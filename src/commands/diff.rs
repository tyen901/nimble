use crate::{md5_digest::Md5Digest, mod_cache::ModCache, repository, srf};
use md5::{Md5, Digest};
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};

#[derive(Debug)]
pub struct DownloadCommand {
    pub file: String,
    pub begin: u64,
    pub end: u64,
}

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
    #[snafu(display("SRF deserialization failure: {}", source))]
    SrfDeserialization { source: serde_json::Error },
    #[snafu(display("Legacy SRF deserialization failure: {}", source))]
    LegacySrfDeserialization { source: srf::Error },
    #[snafu(display("Failed to generate SRF: {}", source))]
    SrfGeneration { source: srf::Error },
}

pub fn diff_repo<'a>(
    mod_cache: &ModCache,
    remote_repo: &'a repository::Repository,
) -> Vec<&'a repository::Mod> {
    let mut downloads = Vec::new();

    // Include both required_mods and optional_mods
    for r#mod in remote_repo.required_mods.iter().chain(remote_repo.optional_mods.iter()) {
        if !mod_cache.mods.contains_key(&r#mod.checksum) {
            downloads.push(r#mod);
        }
    }

    downloads
}

fn verify_file_checksum(path: &Path) -> Result<String, std::io::Error> {
    Ok(Md5Digest::from_file(path)?.to_string())
}

fn normalize_path(path: &str) -> String {
    path.replace('\\', "/").to_lowercase()
}

fn verify_file_exists(base_path: &Path, relative_path: &str) -> bool {
    let full_path = base_path.join(relative_path);
    println!("Checking if file exists: {}", full_path.display());
    full_path.exists()
}

pub fn diff_mod(
    agent: &ureq::Agent,
    repo_base_path: &str,
    local_base_path: &Path,
    remote_mod: &repository::Mod,
) -> Result<Vec<DownloadCommand>, Error> {
    // Get remote SRF first
    let remote_srf_url = repository::make_repo_file_url(
        repo_base_path,
        &format!("{}/mod.srf", remote_mod.mod_name)
    );
    let mut remote_srf = agent
        .get(&remote_srf_url)
        .call()
        .context(HttpSnafu {
            url: remote_srf_url,
        })?
        .into_reader();

    let mut buf = String::new();
    let _len = remote_srf.read_to_string(&mut buf).context(IoSnafu)?;

    let bomless = buf.trim_start_matches('\u{feff}');
    let remote_is_legacy = srf::is_legacy_srf(&mut Cursor::new(bomless)).context(IoSnafu)?;

    let remote_srf: srf::Mod = if remote_is_legacy {
        srf::deserialize_legacy_srf(&mut BufReader::new(Cursor::new(bomless)))
            .context(LegacySrfDeserializationSnafu)?
    } else {
        serde_json::from_str(bomless).context(SrfDeserializationSnafu)?
    };

    let local_path = local_base_path.join(Path::new(&format!("{}/", remote_mod.mod_name)));

    let local_srf = if !local_path.exists() {
        srf::Mod::generate_invalid(&remote_srf)
    } else if local_path.exists() {
        let srf_path = local_path.join(Path::new("mod.srf"));
        let file = File::open(&srf_path);
        match file {
            Ok(file) => {
                let mut reader = BufReader::new(file);
                let srf_result = if srf::is_legacy_srf(&mut reader).context(IoSnafu)? {
                    srf::deserialize_legacy_srf(&mut reader)
                        .context(LegacySrfDeserializationSnafu)
                } else {
                    serde_json::from_reader(&mut reader).context(SrfDeserializationSnafu)
                };

                match srf_result {
                    Ok(srf) => srf,
                    Err(_) => {
                        // If SRF is invalid, rescan the directory
                        println!("Invalid SRF file found for {}, rescanning...", remote_mod.mod_name);
                        srf::scan_mod(&local_path).context(SrfGenerationSnafu)?
                    }
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                println!("No SRF file found for {}, scanning directory...", remote_mod.mod_name);
                srf::scan_mod(&local_path).context(SrfGenerationSnafu)?
            }
            Err(e) => return Err(Error::Io { source: e }),
        }
    } else {
        srf::Mod::generate_invalid(&remote_srf)
    };

    // Add debug logging
    println!("Comparing mod {} (local checksum: {}, remote checksum: {})", 
        remote_mod.mod_name,
        local_srf.checksum,
        remote_srf.checksum
    );

    // Verify checksums match before skipping
    let local_digest = local_srf.checksum.clone();
    let remote_digest = remote_srf.checksum.clone();

    if local_digest == remote_digest 
        && local_srf.files.len() == remote_srf.files.len() 
        && local_path.exists() {
        println!("Skipping mod {} - checksums match", remote_mod.mod_name);
        return Ok(vec![]);
    }
    else {
        println!("Checksums don't match, comparing files...");
    }

    let mut local_files = HashMap::new();
    let mut remote_files = HashMap::new();

    for file in &local_srf.files {
        local_files.insert(&file.path, file);
    }

    for file in &remote_srf.files {
        remote_files.insert(&file.path, file);
    }

    let mut download_list = Vec::new();

    for (path, file) in remote_files.drain() {
        let local_file = local_files.remove(path);
        let full_repo_path = repository::make_repo_file_url(
            &repository::normalize_repo_url(&remote_mod.mod_name),
            path.as_str()
        );
        let normalized_path = normalize_path(path.as_str());
        let local_full_path = local_path.join(&normalized_path);
        
        println!("Checking file: {} at {}", path, local_full_path.display());
        
        match local_file {
            Some(local_file) => {
                if file.checksum != local_file.checksum {
                    if (!verify_file_exists(&local_path, &normalized_path)) {
                        println!("Local file not found at {}", local_full_path.display());
                        download_list.push(DownloadCommand {
                            file: full_repo_path,
                            begin: 0,
                            end: file.length,
                        });
                    } else {
                        match verify_file_checksum(&local_full_path) {
                            Ok(actual_checksum) if actual_checksum == file.checksum => {
                                println!("File {} exists with correct checksum, skipping", path);
                                continue;
                            }
                            Ok(actual_checksum) => {
                                println!("File {} has incorrect checksum: {} (expected {})", 
                                    path, actual_checksum, file.checksum);
                                download_list.push(DownloadCommand {
                                    file: full_repo_path,
                                    begin: 0,
                                    end: file.length,
                                });
                            }
                            Err(e) => {
                                println!("Failed to verify checksum for {}: {}", path, e);
                                download_list.push(DownloadCommand {
                                    file: full_repo_path,
                                    begin: 0,
                                    end: file.length,
                                });
                            }
                        }
                    }
                }
            }
            None => {
                if !verify_file_exists(&local_path, &normalized_path) {
                    println!("File {} missing", path);
                    download_list.push(DownloadCommand {
                        file: full_repo_path,
                        begin: 0,
                        end: file.length,
                    });
                } else {
                    match verify_file_checksum(&local_full_path) {
                        Ok(actual_checksum) if actual_checksum == file.checksum => {
                            println!("File {} exists with correct checksum, skipping", path);
                            continue;
                        }
                        Ok(actual_checksum) => {
                            println!("File {} exists but has wrong checksum: expected {}, found {}", 
                                path, file.checksum, actual_checksum);
                            download_list.push(DownloadCommand {
                                file: full_repo_path,
                                begin: 0,
                                end: file.length,
                            });
                        }
                        Err(e) => {
                            println!("Failed to verify checksum for {}: {}", path, e);
                            download_list.push(DownloadCommand {
                                file: full_repo_path,
                                begin: 0,
                                end: file.length,
                            });
                        }
                    }
                }
            }
        }
    }

    // Only remove leftover files if they're PBOs or don't exist
    remove_leftover_files(local_base_path, &remote_srf, local_files.into_values())
        .context(IoSnafu)?;

    Ok(download_list)
}

fn remove_leftover_files<'a>(
    local_base_path: &Path,
    r#mod: &srf::Mod,
    files: impl Iterator<Item = &'a srf::File>,
) -> Result<(), std::io::Error> {
    for file in files {
        let path = local_base_path.join(&r#mod.name).join(file.path.to_string());

        println!("removing leftover file {}", &path.display());

        if path.exists() {
            if let Err(e) = std::fs::remove_file(&path) {
                eprintln!("Warning: Failed to remove file {}: {}", path.display(), e);
            }
        }
    }

    Ok(())
}
