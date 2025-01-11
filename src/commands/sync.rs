use crate::commands::gen_srf::{gen_srf_for_mod, open_cache_or_gen_srf};
use crate::gui::state::CommandMessage;
use crate::mod_cache::ModCache;
use crate::{repository, srf};
use indicatif::{ProgressBar, ProgressState, ProgressStyle, MultiProgress};
use snafu::{ResultExt, Snafu};
use std::fs::File;
use std::io::{self, BufWriter, Read, Seek, SeekFrom, Write, Cursor, BufReader};
use std::path::Path;
use tempfile::tempfile;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::sync::mpsc::Sender;

use super::diff::{self};
use super::types::{DownloadCommand, DeleteCommand};  // Use shared types
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
    #[snafu(display("Failed to deserialize SRF: {}", source))]
    SrfDeserialization { source: srf::Error },
    #[snafu(display("Failed to serialize data: {}", source))]
    Serialization { source: serde_json::Error },
}

impl From<diff::Error> for Error {
    fn from(err: diff::Error) -> Self {
        Error::Diff { source: err }
    }
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
    println!("Generating SRF files for updated mods...");
    for r#mod in mods {
        // Only generate SRF for mods that were changed
        let mod_path = base_path.join(&r#mod.mod_name);
        let srf_path = mod_path.join("mod.srf");
        
        // Skip if SRF exists and is valid
        if srf_path.exists() {
            if let Ok(file) = std::fs::File::open(&srf_path) {
                if let Ok(existing_srf) = serde_json::from_reader(file) {
                    println!("Skipping SRF generation for unchanged mod {}", r#mod.mod_name);
                    mod_cache.insert(existing_srf);
                    continue;
                }
            }
        }

        println!("Generating SRF for {}", r#mod.mod_name);
        let srf = gen_srf_for_mod(&mod_path, None);
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

fn download_srf_part(
    agent: &ureq::Agent, 
    url: &str,
    range: Option<(u64, u64)>
) -> Result<String, Error> {
    let mut request = agent.get(url);
    
    if let Some((start, end)) = range {
        request = request.set("Range", &format!("bytes={}-{}", start, end));
    }

    let mut response = request.call().context(HttpSnafu { url: url.to_string() })?;
    let mut buf = String::new();
    response.into_reader().read_to_string(&mut buf).context(IoSnafu)?;
    Ok(buf)
}

struct DownloadedSrf {
    mod_name: String,
    srf_data: srf::Mod,
}

#[derive(Debug)]
enum ChecksumExtractionError {
    NoJsonStart,
    NoChecksumField,
    MalformedChecksum(String),
    InvalidLength(usize),
    NonHexCharacters,
}

impl std::fmt::Display for ChecksumExtractionError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::NoJsonStart => write!(f, "Data does not start with a JSON object"),
            Self::NoChecksumField => write!(f, "Could not find checksum field"),
            Self::MalformedChecksum(details) => write!(f, "Malformed checksum: {}", details),
            Self::InvalidLength(len) => write!(f, "Invalid checksum length: {} (expected 32)", len),
            Self::NonHexCharacters => write!(f, "Checksum contains non-hexadecimal characters"),
        }
    }
}

fn extract_checksum(json: &str) -> Result<String, ChecksumExtractionError> {
    if !json.starts_with('{') {
        return Err(ChecksumExtractionError::NoJsonStart);
    }

    // Look for "Checksum":" pattern with optional whitespace
    let checksum_patterns = [
        r#""Checksum":""#,
        r#""checksum":""#,
        r#""Checksum": ""#,
        r#""checksum": ""#,
    ];

    for pattern in checksum_patterns {
        if let Some(start_pos) = json.find(pattern) {
            let quote_pos = start_pos + pattern.len();
            if let Some(end_quote) = json[quote_pos..].find('"') {
                let checksum = &json[quote_pos..quote_pos + end_quote];
                
                // Validate checksum format
                if checksum.len() != 32 {
                    return Err(ChecksumExtractionError::InvalidLength(checksum.len()));
                }
                
                if !checksum.chars().all(|c| c.is_ascii_hexdigit()) {
                    return Err(ChecksumExtractionError::NonHexCharacters);
                }

                // Check for any suspicious patterns that might indicate JSON corruption
                if checksum.contains('{') || checksum.contains('}') || checksum.contains('[') || checksum.contains(']') {
                    return Err(ChecksumExtractionError::MalformedChecksum(
                        "Contains invalid JSON characters".to_string()
                    ));
                }

                return Ok(checksum.to_string());
            }
        }
    }
    
    Err(ChecksumExtractionError::NoChecksumField)
}

fn download_remote_srf(
    agent: &ureq::Agent,
    repo_url: &str,
    mod_name: &str,
    partial: bool,
) -> Result<(srf::Mod, bool), Error> {
    let remote_srf_url = repository::make_repo_file_url(
        repo_url,
        &format!("{}/mod.srf", mod_name)
    );

    if partial {
        println!("Downloading partial SRF for {}", mod_name);
        // Get first 256 bytes which should contain the checksum
        let buf = download_srf_part(agent, &remote_srf_url, Some((0, 255)))?;
        let bomless = buf.trim_start_matches('\u{feff}');

        match extract_checksum(bomless) {
            Ok(checksum) => {
                match Md5Digest::new(&checksum) {
                    Ok(checksum) => {
                        let partial_srf = srf::Mod {
                            name: mod_name.to_string(),
                            checksum: checksum.clone(),
                            files: vec![],
                        };
                        
                        println!("Successfully extracted checksum {} from partial SRF for {}", 
                            checksum, mod_name);
                        return Ok((partial_srf, true));
                    },
                    Err(e) => {
                        println!("Invalid MD5 format in partial SRF for {}: {}", mod_name, e);
                    }
                }
            },
            Err(e) => {
                println!("Failed to extract checksum from partial SRF for {}: {}", mod_name, e);
            }
        }

        println!("Could not find valid checksum in partial data for {}, downloading full SRF", mod_name);
    }

    download_full_srf(agent, &remote_srf_url, mod_name)
}

fn download_full_srf(
    agent: &ureq::Agent,
    remote_srf_url: &str,
    mod_name: &str,
) -> Result<(srf::Mod, bool), Error> {
    println!("Downloading full SRF for {}", mod_name);
    let buf = download_srf_part(agent, remote_srf_url, None)?;
    let bomless = buf.trim_start_matches('\u{feff}');
    let remote_is_legacy = srf::is_legacy_srf(&mut Cursor::new(bomless)).context(IoSnafu)?;

    if remote_is_legacy {
        srf::deserialize_legacy_srf(&mut BufReader::new(Cursor::new(bomless)))
            .context(SrfDeserializationSnafu)
            .map(|srf| (srf, false))
    } else {
        serde_json::from_str(bomless)
            .context(SerializationSnafu)
            .map(|srf| (srf, false))
    }
}

fn remove_leftover_files(
    base_path: &Path,
    mod_name: &str,
    delete_commands: Vec<DeleteCommand>
) -> Result<(), Error> {
    for cmd in delete_commands {
        let path = base_path.join(mod_name).join(cmd.file);
        println!("Removing leftover file {}", &path.display());

        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                eprintln!("Warning: Failed to remove file {}: {}", path.display(), e);
            }
        }
    }
    Ok(())
}

fn process_mod_diff(
    agent: &ureq::Agent,
    repo_url: &str,
    base_path: &Path,
    r#mod: &repository::Mod,
    remote_srf: srf::Mod,
    force_sync: bool,
) -> Result<(Vec<DownloadCommand>, Option<DownloadedSrf>), Error> {
    let (downloads, deletes) = diff::diff_mod(base_path, r#mod, &remote_srf, force_sync)?;
    
    // Handle file deletions first
    remove_leftover_files(base_path, &r#mod.mod_name, deletes)?;

    if !downloads.is_empty() {
        println!("Mod {} needs {} file(s) updated", r#mod.mod_name, downloads.len());
        Ok((downloads, Some(DownloadedSrf {
            mod_name: r#mod.mod_name.clone(),
            srf_data: remote_srf,
        })))
    } else {
        Ok((vec![], None))
    }
}

fn save_srf_files(base_path: &Path, downloaded_srfs: &[DownloadedSrf]) -> Result<(), Error> {
    for srf in downloaded_srfs {
        let mod_path = base_path.join(&srf.mod_name);
        let srf_path = mod_path.join("mod.srf");
        
        std::fs::create_dir_all(&mod_path).context(IoSnafu)?;
        let file = File::create(&srf_path).context(IoSnafu)?;
        serde_json::to_writer(file, &srf.srf_data).context(SerializationSnafu)?;
        println!("Saved updated SRF for {}", srf.mod_name);
    }
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

    // Download all partial SRF files upfront
    let mut partial_srfs = Vec::new();
    for r#mod in &remote_repo.required_mods {
        println!("Downloading SRF for {}", r#mod.mod_name);
        let (srf, partial) = download_remote_srf(agent, repo_url, &r#mod.mod_name, true)?;
        partial_srfs.push((r#mod, srf, partial));
    }

    // Convert scan results into download commands
    let mut download_commands = vec![];
    let mut failed_mods: Vec<String> = Vec::new();
    let mut downloaded_srfs = Vec::new();

    for (r#mod, srf, partial) in partial_srfs {
        let mut needs_full_diff = force_sync;
        let mut diff_result = None;
        let mut remote_srf = None;

        if !force_sync && partial {
            match diff::quick_diff(base_path, r#mod, &srf)? {
                diff::QuickDiffResult::UpToDate => continue,
                diff::QuickDiffResult::NeedsFull => {
                    needs_full_diff = true;
                },
            }
        }

        if needs_full_diff {
            let (full_srf, _) = download_remote_srf(agent, repo_url, &r#mod.mod_name, false)?;
            remote_srf = Some(full_srf.clone());  // Always store full SRF
            diff_result = Some(process_mod_diff(agent, repo_url, base_path, r#mod, full_srf, force_sync)?);
        }

        // Handle any diff results
        if let Some((downloads, _)) = diff_result {
            if !downloads.is_empty() {
                download_commands.extend(downloads);
                // Only store SRF if we have a full version
                if let Some(srf) = remote_srf {
                    downloaded_srfs.push(DownloadedSrf {
                        mod_name: r#mod.mod_name.clone(),
                        srf_data: srf,
                    });
                }
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
            println!("Downloads completed");
            
            // Save updated SRF files
            save_srf_files(base_path, &downloaded_srfs)?;
            
            // Update repository info in cache
            mod_cache.repository = Some(remote_repo.clone());
            mod_cache.last_sync = Some(chrono::Utc::now());
            mod_cache.to_disk(base_path).context(ModCacheOpenSnafu)?;

            if !failed_mods.is_empty() {
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
