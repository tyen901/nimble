use crate::commands::gen_srf::{gen_srf_for_mod, open_cache_or_gen_srf};
use crate::gui::state::CommandMessage;
use crate::mod_cache::ModCache;
use crate::{repository, srf};
use indicatif::{ProgressBar, ProgressState, ProgressStyle, MultiProgress};
use snafu::{ResultExt, Snafu};
use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::Path;
use tempfile::tempfile;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use super::diff::{self, DownloadCommand};
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
    #[snafu(display("Failed to fetch repository info: {}", source))]
    RepositoryFetch { source: repository::Error },
    #[snafu(display("Failed to open ModCache: {}", source))]
    ModCacheOpen { source: crate::mod_cache::Error },
    #[snafu(display("Diff error: {}", source))]
    Diff { source: diff::Error },
    #[snafu(display("Sync was cancelled"))]
    Cancelled,
    #[snafu(display("Failed to serialize cache: {}", source))]
    CacheSerialization { source: serde_json::Error },
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

fn create_progress_bar(total_size: u64) -> ProgressBar {
    let pb = ProgressBar::new(total_size);
    pb.set_style(ProgressStyle::with_template("{spinner:.green} [{elapsed_precise}] [{wide_bar:.cyan/blue}] {bytes}/{total_bytes} ({eta})")
        .unwrap()
        .with_key("eta", |state: &ProgressState, w: &mut dyn std::fmt::Write| write!(w, "{:.1}s", state.eta().as_secs_f64()).unwrap())
        .progress_chars("#>-"));
    pb
}

fn download_file(
    agent: &ureq::Agent,
    remote_url: &str,
    temp_file: &mut File,
    context: &SyncContext,
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
    let mut buffer = vec![0; 8192];

    while let Ok(n) = reader.read(&mut buffer) {
        if n == 0 { break; }
        if context.cancel.load(Ordering::SeqCst) {
            return Err(Error::Cancelled);
        }

        temp_file.write_all(&buffer[..n]).context(IoSnafu)?;
        downloaded += n;
        progress_callback(downloaded as u64, total_size);
    }

    Ok(total_size)
}

fn update_mod_cache(base_path: &Path, mods: &[&repository::Mod], mod_cache: &mut ModCache) -> Result<(), Error> {
    println!("Generating SRF files for downloaded mods...");
    for r#mod in mods {
        println!("Generating SRF for {}", r#mod.mod_name);
        let srf = gen_srf_for_mod(&base_path.join(Path::new(&r#mod.mod_name)), None);
        mod_cache.insert(srf);
    }

    println!("Updating mod cache...");
    let writer = BufWriter::new(File::create(base_path.join("nimble-cache.json")).context(IoSnafu)?);
    serde_json::to_writer(writer, &mod_cache).context(CacheSerializationSnafu)?;
    Ok(())
}

fn execute_command_list(
    agent: &mut ureq::Agent,
    remote_base: &str,
    local_base: &Path,
    commands: Vec<DownloadCommand>,
    context: &SyncContext,
) -> Result<(), Error> {
    let multi = MultiProgress::new();
    let total = commands.len();
    let overall_bar = multi.add(ProgressBar::new(total as u64));
    overall_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} files ({eta})")
            .unwrap()
    );

    for (i, command) in commands.into_iter().enumerate() {
        if context.cancel.load(Ordering::SeqCst) {
            return Err(Error::Cancelled);
        }

        println!("downloading {} of {} - {}", i, total, command.file);

        if let Some(sender) = &context.status_sender {
            sender.send(CommandMessage::SyncProgress {
                file: command.file.clone(),
                progress: 0.0,
                processed: i,
                total: total,
            }).ok();
        }

        let mut temp_download_file = tempfile().context(IoSnafu)?;
        let remote_url = repository::make_repo_file_url(remote_base, &command.file);
        
        let file_bar = multi.add(ProgressBar::new(0));
        file_bar.set_style(
            ProgressStyle::default_bar()
                .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {bytes}/{total_bytes} ({eta})")
                .unwrap()
        );
        file_bar.set_message(command.file.clone());

        let sender = context.status_sender.clone();
        let file_name = command.file.clone();
        let current_index = i;
        let total_files = total;

        let file_bar_ref = file_bar.clone(); // Clone the progress bar for the closure

        let total_size = download_file(agent, &remote_url, &mut temp_download_file, context, move |downloaded, total| {
            file_bar_ref.set_position(downloaded);
            if let Some(sender) = &sender {
                sender.send(CommandMessage::SyncProgress {
                    file: file_name.clone(),
                    progress: downloaded as f32 / total as f32,
                    processed: current_index,
                    total: total_files,
                }).ok();
            }
        })?;

        file_bar.finish_and_clear();
        overall_bar.inc(1);

        // Write to permanent file
        let file_path = local_base.join(Path::new(&command.file));
        std::fs::create_dir_all(file_path.parent().expect("file_path did not have a parent"))
            .context(IoSnafu)?;
        let mut local_file = File::create(&file_path).context(IoSnafu)?;

        temp_download_file.seek(SeekFrom::Start(0)).context(IoSnafu)?;
        std::io::copy(&mut temp_download_file, &mut local_file).context(IoSnafu)?;
    }

    overall_bar.finish_with_message("All files downloaded");
    Ok(())
}

pub fn sync(
    agent: &mut ureq::Agent,
    repo_url: &str,
    base_path: &Path,
    dry_run: bool,
    force_scan: bool,
) -> Result<(), Error> {
    let context = SyncContext::default();
    sync_with_context(agent, repo_url, base_path, dry_run, force_scan, &context)
}

pub fn sync_with_context(
    agent: &mut ureq::Agent,
    repo_url: &str,
    base_path: &Path,
    dry_run: bool,
    force_sync: bool,
    context: &SyncContext,
) -> Result<(), Error> {
    // If force sync, delete the cache file first
    if force_sync {
        let cache_path = base_path.join("nimble-cache.json");
        if cache_path.exists() {
            println!("Force sync: Deleting cache file");
            if let Err(e) = std::fs::remove_file(&cache_path) {
                eprintln!("Warning: Failed to delete cache file: {}", e);
            }
        }
    }

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
    
    let remote_repo = repository::get_repository_info(agent, repo_url)
        .context(RepositoryFetchSnafu)?;
    check_cancelled()?;

    println!("Retrieved repository information. Version: {}", remote_repo.version);

    // Initialize or load mod cache
    let mut mod_cache = ModCache::from_disk_or_empty(base_path).context(ModCacheOpenSnafu)?;

    // Convert scan results into download commands
    let mut download_commands = vec![];
    let mut failed_mods = Vec::new();

    for r#mod in &remote_repo.required_mods {
        match diff::diff_mod(agent, repo_url, base_path, r#mod, force_sync).context(DiffSnafu) {
            Ok(commands) => {
                if !commands.is_empty() {
                    println!("Mod {} needs {} file(s) updated", r#mod.mod_name, commands.len());
                    download_commands.extend(commands);
                }
            },
            Err(e) => {
                eprintln!("Error diffing mod {}: {}", r#mod.mod_name, e);
                failed_mods.push(r#mod.mod_name.clone());
            }
        }
    }

    println!("Total files to download: {}", download_commands.len());

    if dry_run {
        println!("Dry run completed");
        return Ok(());
    }

    // Execute downloads and update cache
    let res = execute_command_list(agent, repo_url, base_path, download_commands, context);

    match res {
        Ok(()) => {
            println!("Downloads completed successfully");
            
            // Update mod cache for changed mods
            let changed_mods: Vec<&repository::Mod> = remote_repo.required_mods.iter()
                .filter(|m| failed_mods.iter().all(|f| f != &m.mod_name))
                .collect();
            
            update_mod_cache(base_path, &changed_mods, &mut mod_cache)?;
            
            // Update repository info in cache
            mod_cache.repository = Some(remote_repo.clone());
            mod_cache.last_sync = Some(chrono::Utc::now());
            mod_cache.to_disk(base_path).context(ModCacheOpenSnafu)?;

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
