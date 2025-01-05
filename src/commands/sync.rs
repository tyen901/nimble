use crate::commands::gen_srf::{gen_srf_for_mod, open_cache_or_gen_srf};
use crate::gui::state::CommandMessage;
use crate::mod_cache::ModCache;
use crate::{repository, srf};
use indicatif::{ProgressBar, ProgressState, ProgressStyle};
use snafu::{ResultExt, Snafu};
use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use tempfile::tempfile;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use super::diff::{self, DownloadCommand};

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
    #[snafu(display("Failed to fetch repository info: {}", source))]
    RepositoryFetch { source: repository::Error },
    #[snafu(display("Failed to open ModCache: {}", source))]
    ModCacheOpen { source: crate::mod_cache::Error },
    #[snafu(display("Diff error: {}", source))]
    Diff { source: diff::Error },
    #[snafu(display("Sync was cancelled"))]
    Cancelled,
}

pub struct SyncContext {
    pub cancel: Arc<AtomicBool>,
    pub status_sender: Option<Sender<CommandMessage>>,
}

impl Default for SyncContext {
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
    commands: &[DownloadCommand],
    context: &SyncContext,
) -> Result<(), Error> {
    for (i, command) in commands.iter().enumerate() {
        if context.cancel.load(Ordering::SeqCst) {
            return Err(Error::Cancelled);
        }

        println!("downloading {} of {} - {}", i, commands.len(), command.file);

        // Report file progress to GUI
        if let Some(sender) = &context.status_sender {
            sender.send(CommandMessage::SyncProgress {
                file: command.file.clone(),
                progress: 0.0, // Start at 0 for new file
                processed: i,
                total: commands.len(),
            }).ok();
        }

        // Download into temp file first
        let mut temp_download_file = tempfile().context(IoSnafu)?;
        let remote_url = format!("{}{}", remote_base, command.file);
        let response = agent.get(&remote_url).call().context(HttpSnafu {
            url: remote_url.clone(),
        })?;

        let total_size = response
            .header("Content-Length")
            .and_then(|s| s.parse::<u64>().ok())
            .unwrap_or(0);

        let pb = ProgressBar::new(total_size);
        pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
            .unwrap()
            .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
            .progress_chars("#>-"));

        let mut reader = response.into_reader();
        let mut downloaded = 0;
        let mut buffer = vec![0; 8192];

        loop {
            if context.cancel.load(Ordering::SeqCst) {
                pb.finish_and_clear();
                return Err(Error::Cancelled);
            }

            match reader.read(&mut buffer) {
                Ok(0) => break, // EOF
                Ok(n) => {
                    temp_download_file.write_all(&buffer[..n]).context(IoSnafu)?;
                    downloaded += n;
                    pb.set_position(downloaded as u64);

                    // Update progress
                    if let Some(sender) = &context.status_sender {
                        sender.send(CommandMessage::SyncProgress {
                            file: command.file.clone(),
                            progress: downloaded as f32 / total_size as f32,
                            processed: i,
                            total: commands.len(),
                        }).ok();
                    }
                }
                Err(e) => {
                    pb.finish_and_clear();
                    return Err(Error::Io { source: e });
                }
            }
        }

        pb.finish_and_clear();

        // Write to permanent file
        let file_path = local_base.join(Path::new(&command.file));
        std::fs::create_dir_all(file_path.parent().expect("file_path did not have a parent"))
            .context(IoSnafu)?;
        let mut local_file = File::create(&file_path).context(IoSnafu)?;

        temp_download_file.seek(SeekFrom::Start(0)).context(IoSnafu)?;
        std::io::copy(&mut temp_download_file, &mut local_file).context(IoSnafu)?;
    }

    Ok(())
}

pub fn sync(
    agent: &mut ureq::Agent,
    repo_url: &str,
    base_path: &Path,
    dry_run: bool,
) -> Result<(), Error> {
    let context = SyncContext::default();
    sync_with_context(agent, repo_url, base_path, dry_run, &context)
}

pub fn sync_with_context(
    agent: &mut ureq::Agent,
    repo_url: &str,
    base_path: &Path,
    dry_run: bool,
    context: &SyncContext,
) -> Result<(), Error> {
    // Check cancel flag at each major step
    let check_cancelled = || {
        if context.cancel.load(Ordering::SeqCst) {
            return Err(Error::Cancelled);
        }
        Ok(())
    };

    if let Some(sender) = &context.status_sender {
        sender.send(CommandMessage::ScanningStatus("Fetching repository information...".into())).ok();
    }
    check_cancelled()?;

    println!("Starting sync process from {}", repo_url);
    
    let remote_repo = repository::get_repository_info(agent, &format!("{repo_url}/repo.json"))
        .context(RepositoryFetchSnafu)?;
    check_cancelled()?;

    println!("Retrieved repository information. Version: {}", remote_repo.version);

    if let Some(sender) = &context.status_sender {
        sender.send(CommandMessage::ScanningStatus("Scanning local files...".into())).ok();
    }
    check_cancelled()?;

    let mut mod_cache = open_cache_or_gen_srf(base_path).context(ModCacheOpenSnafu)?;
    let check = diff::diff_repo(&mod_cache, &remote_repo);
    check_cancelled()?;

    println!("Found {} mod(s) that need updating", check.len());

    // remove all mods to check from cache, we'll read them later
    for r#mod in &check {
        mod_cache.remove(&r#mod.checksum);
    }

    let mut download_commands = vec![];
    let mut failed_mods = Vec::new();

    for r#mod in &check {
        println!("Checking mod: {}", r#mod.mod_name);
        match diff::diff_mod(agent, repo_url, base_path, r#mod).context(DiffSnafu) {
            Ok(commands) => {
                println!("  - Found {} file(s) to update", commands.len());
                download_commands.extend(commands);
            },
            Err(e) => {
                eprintln!("Error diffing mod {}: {}", r#mod.mod_name, e);
                failed_mods.push(r#mod.mod_name.clone());
                continue;
            }
        }
    }

    println!("Total files to download: {}", download_commands.len());

    if dry_run {
        println!("Dry run completed");
        return Ok(());
    }

    let res = execute_command_list(agent, repo_url, base_path, &download_commands, context);

    match res {
        Ok(()) => {
            println!("Downloads completed successfully");
            
            // gen_srf for the mods we downloaded
            println!("Generating SRF files for downloaded mods...");
            for r#mod in &check {
                println!("  - Generating SRF for {}", r#mod.mod_name);
                let srf = gen_srf_for_mod(&base_path.join(Path::new(&r#mod.mod_name)), None);
                mod_cache.insert(srf);
            }

            // reserialize the cache
            println!("Updating mod cache...");
            let writer = BufWriter::new(File::create(base_path.join("nimble-cache.json")).unwrap());
            serde_json::to_writer(writer, &mod_cache).unwrap();
            
            if (!failed_mods.is_empty()) {
                println!("Sync completed with some failures:");
                for failed in failed_mods {
                    println!("  - Failed to sync: {}", failed);
                }
            } else {
                println!("Sync completed successfully!");
            }
            Ok(())
        },
        Err(Error::Cancelled) => {
            println!("Sync was cancelled by user");
            Err(Error::Cancelled)
        },
        Err(e) => {
            println!("Sync failed: {}", e);
            println!("You should retry the sync");
            Err(e)
        }
    }
}
