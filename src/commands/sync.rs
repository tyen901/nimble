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
use std::sync::Mutex;
use std::sync::mpsc::Sender;
use rayon::iter::IntoParallelIterator;
use rayon::iter::ParallelIterator;
use crossbeam_channel::{bounded, Sender as CbSender, Receiver as CbReceiver};
use rayon::prelude::*;

use super::diff::{self};
use super::types::{DownloadCommand, DeleteCommand};  // Use shared types
use crate::md5_digest::Md5Digest;
use super::download::{self, DownloadContext};

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

#[derive(Clone)]
pub struct SyncContext {
    pub download: DownloadContext,
}

impl Default for SyncContext {
    fn default() -> Self {
        Self {
            download: DownloadContext::default(),
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

fn download_srf_part(
    agent: &ureq::Agent, 
    url: &str,
    range: Option<(u64, u64)>
) -> Result<String, Error> {
    let mut request = agent.get(url);
    
    if let Some((start, end)) = range {
        request = request.set("Range", &format!("bytes={}-{}", start, end));
    }

    let response = request.call().context(HttpSnafu { url: url.to_string() })?;
    let mut buf = String::new();
    response.into_reader().read_to_string(&mut buf).context(IoSnafu)?;
    Ok(buf)
}

struct DownloadedSrf {
    mod_name: String,
    srf_data: srf::Mod,
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

        match diff::extract_checksum(bomless) {  // Use the one from diff module
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
    remove_leftover_files(base_path, r#mod.mod_name.as_str(), deletes)?;

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
        if context.download.cancel.load(Ordering::SeqCst) {
            return Err(Error::Cancelled);
        }
        Ok(())
    };

    if let Some(sender) = &context.download.status_sender {
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

    let partial_srfs: Result<Vec<_>, Error> = remote_repo.required_mods.iter().par_bridge()
        .map(|r#mod| -> Result<_, Error> {
            println!("Downloading SRF for {}", r#mod.mod_name);
            let (srf, partial) = download_remote_srf(agent, repo_url, &r#mod.mod_name, true)?;
            Ok((r#mod, srf, partial))
        })
        .collect();
    
    let partial_srfs = partial_srfs?;

    // Process mods in parallel
    let results: Result<Vec<_>, Error> = partial_srfs.par_iter()
        .map(|(r#mod, srf, partial)| -> Result<_, Error> {
            let mut needs_full_diff = force_sync;
            let mut diff_result = None;
            let mut remote_srf = None;

            if !force_sync && *partial {
                match diff::quick_diff(base_path, r#mod, &srf)? {
                    diff::QuickDiffResult::UpToDate => return Ok((vec![], None)),
                    diff::QuickDiffResult::NeedsFull => {
                        needs_full_diff = true;
                    },
                }
            }

            if needs_full_diff {
                let (full_srf, _) = download_remote_srf(agent, repo_url, &r#mod.mod_name, false)?;
                remote_srf = Some(full_srf.clone());
                diff_result = Some(process_mod_diff(agent, repo_url, base_path, r#mod, full_srf, force_sync)?);
            }

            // Handle diff results
            if let Some((downloads, _)) = diff_result {
                if !downloads.is_empty() {
                    return Ok((downloads, remote_srf.map(|srf| DownloadedSrf {
                        mod_name: r#mod.mod_name.clone(),
                        srf_data: srf,
                    })));
                }
            }
            
            Ok((vec![], None))
        })
        .collect();

    let results = results?;

    // Combine results
    let mut download_commands = Vec::new();
    let mut downloaded_srfs = Vec::new();
    
    for (downloads, srf_opt) in results {
        download_commands.extend(downloads);
        if let Some(srf) = srf_opt {
            downloaded_srfs.push(srf);
        }
    }

    println!("Total files to download: {}", download_commands.len());

    if dry_run {
        println!("Dry run completed");
        return Ok(());
    }

    // Execute downloads and update cache
    let res = download::download_files(
        agent, 
        repo_url, 
        base_path, 
        download_commands, 
        context.download.clone()
    ).map_err(|e| match e {
        download::Error::Cancelled => Error::Cancelled,
        e => Error::Io { source: std::io::Error::new(std::io::ErrorKind::Other, e.to_string()) },
    })?;

    println!("Downloads completed");
    
    // Save updated SRF files
    save_srf_files(base_path, &downloaded_srfs)?;
    
    // Update repository info in cache
    mod_cache.repository = Some(remote_repo.clone());
    mod_cache.last_sync = Some(chrono::Utc::now());
    mod_cache.to_disk(base_path).context(ModCacheOpenSnafu)?;

    println!("Sync completed successfully!");
    Ok(())
}
