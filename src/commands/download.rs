use crate::commands::gen_srf::{gen_srf_for_mod, open_cache_or_gen_srf};
use crate::gui::state::CommandMessage;
use crate::mod_cache::ModCache;
use crate::{repository, srf};
use snafu::{ResultExt, Snafu};
use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write, Cursor, BufReader};
use std::path::Path;
use tempfile::tempfile;
use std::sync::atomic::{AtomicBool, Ordering, AtomicU64};
use std::sync::Arc;
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use rayon::iter::{IntoParallelIterator, ParallelIterator};
use crossbeam_channel::{bounded, Sender as CbSender, Receiver as CbReceiver};

use super::diff::{self};
use super::types::{DownloadCommand, DeleteCommand};
use crate::md5_digest::Md5Digest;

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("io error: {}", source))]
    Io { source: std::io::Error },
    #[snafu(display("Error while requesting repository data: {}", source))]
    Http {
        url: String,
        #[snafu(source(from(ureq::Error, Box::new)))]
        source: Box<ureq::Error>,
    },
    #[snafu(display("Sync was cancelled"))]
    Cancelled,
}

#[derive(Clone)]
pub struct DownloadContext {
    pub cancel: Arc<AtomicBool>,
    pub status_sender: Option<Sender<CommandMessage>>,
}

impl Default for DownloadContext {
    fn default() -> Self {
        Self {
            cancel: Arc::new(AtomicBool::new(false)),
            status_sender: None,
        }
    }
}

fn execute_command_list(
    agent: &mut ureq::Agent,
    remote_base: &str,
    local_base: &Path,
    commands: Vec<DownloadCommand>,
    context: DownloadContext,
) -> Result<(), Error> {
    let total_files = commands.len();
    let total_bytes: u64 = commands.iter().map(|cmd| cmd.end - cmd.begin).sum();
    
    println!("Starting download of {} files ({} bytes total)...", total_files, total_bytes);
    
    let bytes_downloaded = Arc::new(AtomicU64::new(0));
    let files_completed = Arc::new(AtomicU64::new(0));

    // Create thread-safe agent pool
    const MAX_CONCURRENT_DOWNLOADS: usize = 4;
    let agent_pool = Arc::new(Mutex::new(vec![
        agent.clone(),
        ureq::AgentBuilder::new().build(),
        ureq::AgentBuilder::new().build(),
        ureq::AgentBuilder::new().build(),
    ]));

    let context = Arc::new(context);
    let (work_tx, work_rx): (CbSender<DownloadCommand>, CbReceiver<DownloadCommand>) = 
        bounded(MAX_CONCURRENT_DOWNLOADS * 2);
    let (result_tx, result_rx): (CbSender<Result<(), Error>>, CbReceiver<Result<(), Error>>) = 
        bounded(commands.len());

    // Spawn worker threads
    let mut workers = Vec::new();
    for worker_id in 0..MAX_CONCURRENT_DOWNLOADS {
        let work_rx = work_rx.clone();
        let agent_pool = agent_pool.clone();
        let context = context.clone();
        let bytes_downloaded = bytes_downloaded.clone();
        let files_completed = files_completed.clone();
        let result_tx = result_tx.clone();
        let remote_base = remote_base.to_string();
        let local_base = local_base.to_path_buf();
        
        workers.push(std::thread::spawn(move || {
            while let Ok(command) = work_rx.recv() {
                if context.cancel.load(Ordering::SeqCst) {
                    break;
                }

                let mut agent_guard = agent_pool.lock().unwrap();
                let agent = agent_guard.pop().unwrap_or_else(|| ureq::AgentBuilder::new().build());
                drop(agent_guard);

                println!("[Worker {}] Starting download: {}", worker_id, command.file);
                
                let result = (|| {
                    let mut temp_download_file = tempfile().context(IoSnafu)?;
                    let remote_url = repository::make_repo_file_url(&remote_base, &command.file);
                    
                    let progress_callback = {
                        let bytes_downloaded = bytes_downloaded.clone();
                        let context = context.clone();
                        let file_name = command.file.clone();
                        let files_completed = files_completed.clone();

                        move |chunk: u64, _: u64| {
                            bytes_downloaded.fetch_add(chunk, Ordering::Relaxed);
                            
                            if let Some(sender) = &context.status_sender {
                                sender.send(CommandMessage::SyncProgress {
                                    file: file_name.clone(),
                                    progress: bytes_downloaded.load(Ordering::Relaxed) as f32 / total_bytes as f32,
                                    processed: files_completed.load(Ordering::Relaxed) as usize,
                                    total: total_files,
                                }).ok();
                            }
                        }
                    };

                    download_file(&agent, &remote_url, &mut temp_download_file, &context, progress_callback)?;

                    let file_path = local_base.join(Path::new(&command.file));
                    std::fs::create_dir_all(file_path.parent().expect("file_path did not have a parent"))
                        .context(IoSnafu)?;
                    let mut local_file = File::create(&file_path).context(IoSnafu)?;

                    temp_download_file.seek(SeekFrom::Start(0)).context(IoSnafu)?;
                    std::io::copy(&mut temp_download_file, &mut local_file).context(IoSnafu)?;

                    files_completed.fetch_add(1, Ordering::Relaxed);
                    println!("[Worker {}] Completed download: {}", worker_id, command.file);
                    Ok(())
                })();

                agent_pool.lock().unwrap().push(agent);
                result_tx.send(result).ok();
            }
        }));
    }

    // Submit work
    for command in commands {
        if let Err(_) = work_tx.send(command) {
            println!("Warning: Failed to queue download, channel closed");
            break;
        }
    }
    drop(work_tx);

    // Process downloads and keep progress bars active in main thread
    // Collect results
    let mut errors = Vec::new();
    for _ in 0..total_files {
        match result_rx.recv() {
            Ok(Ok(())) => (),
            Ok(Err(e)) => errors.push(e),
            Err(_) => break,
        }
    }

    // Wait for workers to finish
    for worker in workers {
        worker.join().ok();
    }

    println!("All downloads complete! ({} files, {} bytes)", total_files, bytes_downloaded.load(Ordering::Relaxed));

    // Handle errors
    if !errors.is_empty() {
        if errors.iter().any(|e| matches!(e, Error::Cancelled)) {
            return Err(Error::Cancelled);
        }
        return Err(errors.into_iter().next().unwrap());
    }

    Ok(())
}

fn download_file(
    agent: &ureq::Agent,
    remote_url: &str,
    temp_file: &mut File,
    context: &Arc<DownloadContext>,
    progress_callback: impl Fn(u64, u64),
) -> Result<u64, Error> {
    let response = agent.get(remote_url).call().context(HttpSnafu {
        url: remote_url.to_string(),
    })?;

    let total_size = response
        .header("Content-Length")
        .and_then(|s| s.parse::<u64>().ok())
        .unwrap_or(0);

    let mut reader = response.into_reader();
    let mut downloaded = 0;
    // Increase buffer size for more efficient downloads
    let mut buffer = vec![0; 64 * 1024]; // Use 64KB buffer instead of 8KB

    while let Ok(n) = reader.read(&mut buffer) {
        if n == 0 { break; }
        if context.cancel.load(Ordering::SeqCst) {
            return Err(Error::Cancelled);
        }

        temp_file.write_all(&buffer[..n]).context(IoSnafu)?;
        downloaded += n as u64;
        // Only call progress every chunk to reduce terminal updates
        progress_callback(n as u64, total_size);
    }

    Ok(total_size)
}

pub fn download_files(
    agent: &mut ureq::Agent,
    remote_base: &str,
    local_base: &Path,
    commands: Vec<DownloadCommand>,
    context: DownloadContext,
) -> Result<(), Error> {
    execute_command_list(agent, remote_base, local_base, commands, context)
}