use crate::commands::gen_srf::gen_srf_for_mod;
use crate::mod_cache::ModCache;
use crate::{repository, srf};
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, BufWriter, Cursor, Read, Write};
use std::path::Path;
use tempfile::NamedTempFile;
use rayon::prelude::*;
use rayon::ThreadPoolBuilder;
use std::time::Instant;
use std::sync::atomic::{AtomicBool, Ordering};

pub trait ProgressReporter: Send + Sync {
    fn set_stage(&self, stage: &str);
    fn set_total_files(&self, count: usize, download_size: u64, repo_size: u64);
    fn start_task(&self, filename: &str, total: u64);
    fn update_file_progress(&self, filename: &str, bytes: u64, total: u64, speed: f64);
    fn file_completed(&self, filename: &str);
}

#[derive(Debug)]
struct DownloadCommand {
    file: String,
    begin: u64,
    end: u64,
}

#[derive(Snafu, Debug)]
pub enum Error {
    #[snafu(display("io error: {}", source))]
    Io { source: std::io::Error },
    Template { source: indicatif::style::TemplateError },
    #[snafu(display("Error while requesting repository data: {}", source))]
    Http {
        url: String,

        #[snafu(source(from(ureq::Error, Box::new)))]
        source: Box<ureq::Error>,
    },
    #[snafu(display("Failed to fetch repository info: {}", source))]
    RepositoryFetch { source: repository::Error },
    #[snafu(display("SRF deserialization failure: {}", source))]
    SrfDeserialization { source: serde_json::Error },
    #[snafu(display("Legacy SRF deserialization failure: {}", source))]
    LegacySrfDeserialization { source: srf::Error },
    #[snafu(display("Failed to generate SRF: {}", source))]
    SrfGeneration { source: srf::Error },
    #[snafu(display("Failed to open ModCache: {}", source))]
    ModCacheOpen { source: crate::mod_cache::Error },
}

impl From<indicatif::style::TemplateError> for Error {
    fn from(error: indicatif::style::TemplateError) -> Self {
        Error::Template { source: error }
    }
}

impl From<std::io::Error> for Error {
    fn from(error: std::io::Error) -> Self {
        Error::Io { source: error }
    }
}

const CHUNK_SIZE: usize = 1024 * 1024; // 1MB chunks
const DOWNLOAD_BUFFER_SIZE: usize = 8192 * 16; // 128KB buffer

fn diff_repo<'a>(
    mod_cache: &ModCache,
    remote_repo: &'a repository::Repository,
) -> Vec<&'a repository::Mod> {
    // Pre-allocate with estimated size
    let mut downloads = Vec::with_capacity(remote_repo.required_mods.len());

    // repo checksums use the repo generation timestamp in the checksum calculation, so we can't really
    // generate them for comparison. they aren't that useful anyway

    for r#mod in &remote_repo.required_mods {
        if !mod_cache.mods.contains_key(&r#mod.checksum) {
            downloads.push(r#mod);
        }
    }

    downloads
}

fn diff_mod(
    agent: &ureq::Agent,
    repo_base_path: &str,
    local_base_path: &Path,
    remote_mod: &repository::Mod,
    cancel_flag: &AtomicBool,
) -> Result<Vec<DownloadCommand>, Error> {
    if cancel_flag.load(Ordering::SeqCst) {
        return Err(Error::Io { 
            source: std::io::Error::new(std::io::ErrorKind::Interrupted, "sync cancelled") 
        });
    }

    // HACK HACK: this REALLY should be parsed through streaming rather than through buffering the whole thing
    let remote_srf_url = format!("{}{}/mod.srf", repo_base_path, remote_mod.mod_name);
    let mut remote_srf = agent
        .get(&remote_srf_url)
        .call()
        .context(HttpSnafu {
            url: remote_srf_url,
        })?
        .into_reader();

    let mut buf = String::with_capacity(8192); // Pre-allocate with reasonable size
    let _len = remote_srf.read_to_string(&mut buf).context(IoSnafu)?;

    // yeet utf-8 bom, which is bad, not very useful and not supported by serde
    let bomless = buf.trim_start_matches('\u{feff}');

    let remote_is_legacy = srf::is_legacy_srf(&mut Cursor::new(bomless)).context(IoSnafu)?;

    let remote_srf: srf::Mod = if remote_is_legacy {
        srf::deserialize_legacy_srf(&mut BufReader::new(Cursor::new(bomless)))
            .context(LegacySrfDeserializationSnafu)?
    } else {
        serde_json::from_str(bomless).context(SrfDeserializationSnafu)?
    };

    let local_path = local_base_path.join(Path::new(&format!("{}/", remote_mod.mod_name)));
    let srf_path = local_path.join(Path::new("mod.srf"));

    let local_srf = {
        if local_path.exists() {
            let file = File::open(srf_path);

            match file {
                Ok(file) => {
                    let mut reader = BufReader::new(file);

                    if srf::is_legacy_srf(&mut reader).context(IoSnafu)? {
                        srf::deserialize_legacy_srf(&mut reader)
                            .context(LegacySrfDeserializationSnafu)?
                    } else {
                        serde_json::from_reader(&mut reader).context(SrfDeserializationSnafu)?
                    }
                }
                Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                    srf::scan_mod(&local_path).context(SrfGenerationSnafu)?
                }
                Err(e) => return Err(Error::Io { source: e }),
            }
        } else {
            srf::Mod::generate_invalid(&remote_srf)
        }
    };

    if local_srf.checksum == remote_srf.checksum {
        return Ok(vec![]);
    }

    // Pre-allocate hashmaps with known sizes
    let mut local_files = HashMap::with_capacity(local_srf.files.len());
    let mut remote_files = HashMap::with_capacity(remote_srf.files.len());
    let mut download_list = Vec::with_capacity(remote_srf.files.len());

    for file in &local_srf.files {
        local_files.insert(&file.path, file);
    }

    for file in &remote_srf.files {
        remote_files.insert(&file.path, file);
    }

    for (path, file) in remote_files.drain() {
        let local_file = local_files.remove(path);

        if let Some(local_file) = local_file {
            if file.checksum != local_file.checksum {
                // TODO: implement file diffing. for now, just download everything

                download_list.push(DownloadCommand {
                    file: format!("{}/{}", remote_srf.name, path),
                    begin: 0,
                    end: file.length,
                });
            }
        } else {
            download_list.push(DownloadCommand {
                file: format!("{}/{}", remote_srf.name, path),
                begin: 0,
                end: file.length,
            });
        }
    }

    // remove any local files that remain here
    remove_leftover_files(local_base_path, &remote_srf, local_files.into_values())
        .context(IoSnafu)?;

    Ok(download_list)
}

// remove files that are present in the local disk but not in the remote repo
fn remove_leftover_files<'a>(
    local_base_path: &Path,
    r#mod: &srf::Mod,
    files: impl Iterator<Item = &'a srf::File>,
) -> Result<(), std::io::Error> {
    for file in files {
        let path = file
            .path
            .to_path(local_base_path.join(Path::new(&r#mod.name)));

        println!("removing leftover file {}", &path.display());

        std::fs::remove_file(&path)?;
    }

    Ok(())
}

fn execute_command_list(
    agent: &mut ureq::Agent,
    remote_base: &str,
    local_base: &Path,
    commands: &[DownloadCommand],
    progress: &dyn ProgressReporter,
    threads: usize,
    cancel_flag: &AtomicBool,
) -> Result<(), Error> {
    let pool = ThreadPoolBuilder::new()
        .num_threads(threads)  // Use configured thread count
        .build()
        .unwrap();

    let total_download_size: u64 = commands.iter().map(|c| c.end - c.begin).sum();
    let total_repo_size: u64 = commands.iter().map(|c| c.end).sum();

    progress.set_total_files(commands.len(), total_download_size, total_repo_size);

    pool.install(|| {
        commands.par_iter().try_for_each(|command| {
            if cancel_flag.load(Ordering::SeqCst) {
                return Err(Error::Io { 
                    source: std::io::Error::new(std::io::ErrorKind::Interrupted, "sync cancelled") 
                });
            }
            
            let file_name = Path::new(&command.file).file_name().unwrap().to_str().unwrap();
            let mut last_update = Instant::now();
            let mut last_bytes = 0u64;

            // which will later make us crash in gen_srf
            let mut temp_download_file = NamedTempFile::new().context(IoSnafu)?;
            
            // Pre-allocate file with required size
            temp_download_file.as_file_mut().set_len(command.end).context(IoSnafu)?;

            let remote_url = format!("{}{}", remote_base, command.file);

            let response = agent.get(&remote_url).call().context(HttpSnafu {
                url: remote_url.clone(),
            })?;

            let total_size = response.header("Content-Length")
                .and_then(|len| len.parse().ok())
                .unwrap_or(0);

            progress.start_task(file_name, total_size);

            let mut reader = response.into_reader();

            let mut current_progress = 0u64;
            let mut buffer = vec![0u8; DOWNLOAD_BUFFER_SIZE];
            
            // Use buffered writes
            {
                let mut writer = BufWriter::new(temp_download_file.as_file_mut());
                
                loop {
                    if cancel_flag.load(Ordering::SeqCst) {
                        return Err(Error::Io { 
                            source: std::io::Error::new(std::io::ErrorKind::Interrupted, "sync cancelled") 
                        });
                    }

                    match reader.read(&mut buffer) {
                        Ok(0) => break,
                        Ok(n) => {
                            writer.write_all(&buffer[..n]).context(IoSnafu)?;
                            current_progress += n as u64;
                            
                            // Update speed calculation every 100ms
                            if last_update.elapsed().as_millis() > 100 {
                                let elapsed = last_update.elapsed().as_secs_f64();
                                let bytes_since_last = current_progress - last_bytes;
                                let speed = bytes_since_last as f64 / elapsed;
                                
                                progress.update_file_progress(
                                    file_name,
                                    current_progress,
                                    total_size,
                                    speed
                                );
                                
                                last_update = Instant::now();
                                last_bytes = current_progress;
                            }
                        }
                        Err(e) => return Err(Error::Io { source: e }),
                    }
                }
                
                writer.flush().context(IoSnafu)?;
            } // writer is dropped here, releasing the borrow

            // Only complete the file if we weren't cancelled
            if !cancel_flag.load(Ordering::SeqCst) {
                progress.file_completed(file_name);

                // Move temp file to final location
                let file_path = local_base.join(Path::new(&command.file));
                std::fs::create_dir_all(file_path.parent().expect("file_path did not have a parent"))
                    .context(IoSnafu)?;
                
                std::fs::rename(temp_download_file.path(), &file_path).context(IoSnafu)?;
            }

            Ok(())
        })
    })
}

pub fn sync(
    agent: &mut ureq::Agent,
    repo_url: &str,
    base_path: &Path,
    dry_run: bool,
    progress: &dyn ProgressReporter,
    threads: usize,
    cancel_flag: &AtomicBool,
) -> Result<(), Error> {
    progress.set_stage("Fetching repository info");
    if cancel_flag.load(Ordering::SeqCst) { return Ok(()); }
    
    let remote_repo = repository::get_repository_info(agent, &format!("{repo_url}/repo.json"))
        .context(RepositoryFetchSnafu)?;

    progress.set_stage("Loading mod cache");
    if cancel_flag.load(Ordering::SeqCst) { return Ok(()); }
    
    let mut mod_cache = ModCache::from_disk_or_empty(base_path).context(ModCacheOpenSnafu)?;

    progress.set_stage("Checking for updates");
    let check = diff_repo(&mod_cache, &remote_repo);

    println!("mods to check: {check:#?}");

    // remove all mods to check from cache, we'll read them later
    for r#mod in &check {
        mod_cache.remove(&r#mod.checksum);
    }

    progress.set_stage("Calculating required downloads");
    if cancel_flag.load(Ordering::SeqCst) { return Ok(()); }
    
    // Parallelize mod diffing with cancellation support
    let download_commands: Vec<_> = check.par_iter()
        .filter_map(|m| diff_mod(agent, repo_url, base_path, m, cancel_flag).ok())
        .filter(|r| !cancel_flag.load(Ordering::SeqCst)) // Skip remaining items if cancelled
        .flatten()
        .collect();

    if cancel_flag.load(Ordering::SeqCst) {
        progress.set_stage("Sync cancelled");
        return Ok(());
    }

    println!("download commands: {download_commands:#?}");

    if dry_run {
        return Ok(());
    }

    progress.set_stage("Downloading files");
    if cancel_flag.load(Ordering::SeqCst) { return Ok(()); }
    
    let res = execute_command_list(agent, repo_url, base_path, &download_commands, progress, threads, cancel_flag);

    match res {
        Ok(_) => {
            if !cancel_flag.load(Ordering::SeqCst) {
                // Only update cache if not cancelled
                for r#mod in &check {
                    let srf = gen_srf_for_mod(&base_path.join(Path::new(&r#mod.mod_name)));
                    mod_cache.insert(srf);
                }

                let writer = BufWriter::new(File::create(base_path.join("nimble-cache.json")).unwrap());
                serde_json::to_writer(writer, &mod_cache).unwrap();
            }
            Ok(())
        }
        Err(Error::Io { source }) if source.kind() == std::io::ErrorKind::Interrupted => {
            progress.set_stage("Sync cancelled");
            Ok(())
        }
        Err(e) => {
            progress.set_stage("Error occurred while downloading");
            Err(e)
        }
    }
}
