use crate::{md5_digest::Md5Digest, mod_cache::ModCache, repository, srf};
use super::types::{DownloadCommand, DeleteCommand};
use md5::{Md5, Digest};
use snafu::{ResultExt, Snafu};
use std::collections::HashMap;
use std::fs::File;
use std::io::{BufReader, Cursor, Read};
use std::path::{Path, PathBuf};
use rayon::prelude::*;

#[derive(Debug, Clone)]
pub enum QuickDiffResult {
    UpToDate,
    NeedsFull,
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

pub fn quick_diff(
    local_base_path: &Path,
    remote_mod: &repository::Mod,
    remote_srf: &srf::Mod,
) -> Result<QuickDiffResult, Error> {
    let local_path = local_base_path.join(Path::new(&format!("{}/", remote_mod.mod_name)));
    let srf_path = local_path.join("mod.srf");

    if (!srf_path.exists()) {
        println!("No local SRF found for {}, needs full check", remote_mod.mod_name);
        return Ok(QuickDiffResult::NeedsFull);
    }

    let local_srf = {
        let file = File::open(&srf_path).context(IoSnafu)?;
        let mut reader = BufReader::new(file);
        if srf::is_legacy_srf(&mut reader).context(IoSnafu)? {
            srf::deserialize_legacy_srf(&mut reader).context(LegacySrfDeserializationSnafu)?
        } else {
            serde_json::from_reader(&mut reader).context(SrfDeserializationSnafu)?
        }
    };

    println!("Quick comparing mod {} (local: {}, remote: {})", 
        remote_mod.mod_name,
        local_srf.checksum,
        remote_srf.checksum
    );

    if local_srf.checksum == remote_srf.checksum {
        println!("Quick check passed for {}", remote_mod.mod_name);
        Ok(QuickDiffResult::UpToDate)
    } else {
        println!("Quick check detected changes for {}, needs full check", remote_mod.mod_name);
        Ok(QuickDiffResult::NeedsFull)
    }
}

pub fn diff_mod(
    local_base_path: &Path,
    remote_mod: &repository::Mod,
    remote_srf: &srf::Mod,
    force_scan: bool,
) -> Result<(Vec<DownloadCommand>, Vec<DeleteCommand>), Error> { 
    let local_path = local_base_path.join(Path::new(&format!("{}/", remote_mod.mod_name)));
    let srf_path = local_path.join("mod.srf");

    // If force scan, delete the local SRF file first
    if force_scan && srf_path.exists() {
        println!("Force scanning directory for {}...", remote_mod.mod_name);
        if let Err(e) = std::fs::remove_file(&srf_path) {
            eprintln!("Warning: Failed to delete SRF file: {}", e);
        }
    }

    // Ensure the mod directory exists
    if !local_path.exists() {
        std::fs::create_dir_all(&local_path).context(IoSnafu)?;
    }

    // Generate SRF file if it doesn't exist or force_scan was used
    if !srf_path.exists() {
        println!("No SRF file found for {}, generating initial SRF...", remote_mod.mod_name);
        let initial_srf = if local_path.exists() {
            srf::scan_mod(&local_path).context(SrfGenerationSnafu)?
        } else {
            srf::Mod::generate_invalid(&remote_srf)
        };
        
        // Write the initial SRF file
        let file = File::create(&srf_path).context(IoSnafu)?;
        serde_json::to_writer(file, &initial_srf).context(SrfDeserializationSnafu)?;
    }

    // Now read the local SRF file (which we know exists)
    let local_srf = {
        let file = File::open(&srf_path).context(IoSnafu)?;
        let mut reader = BufReader::new(file);
        if srf::is_legacy_srf(&mut reader).context(IoSnafu)? {
            srf::deserialize_legacy_srf(&mut reader).context(LegacySrfDeserializationSnafu)?
        } else {
            serde_json::from_reader(&mut reader).context(SrfDeserializationSnafu)?
        }
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
        return Ok((vec![], vec![]));
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

    let mut delete_list = Vec::new();
    
    // Add leftover files to delete list
    for (path, _) in local_files {
        delete_list.push(DeleteCommand {
            file: path.as_str().to_string(),
        });
    }

    Ok((download_list, delete_list))
}
