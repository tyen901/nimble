use crate::repository::Repository;
use crate::srf;
use crate::gui::state::CommandMessage;
use relative_path::RelativePathBuf;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::{fs, io};

#[derive(Debug)]
pub struct ModUpdate {
    pub name: String,
    pub files: Vec<FileUpdate>,
}

#[derive(Debug)]
pub struct FileUpdate {
    pub path: RelativePathBuf,
    pub checksum: String,
    pub size: u64,
}

const TEMP_FOLDER: &str = ".nimble_temp";

fn download_remote_srf(
    agent: &mut ureq::Agent,
    repo_url: &str,
    mod_name: &str,
) -> Result<srf::Mod, String> {
    let remote_srf_url = format!("{}/{}/mod.srf", repo_url.trim_end_matches('/'), mod_name);
    
    agent
        .get(&remote_srf_url)
        .call()
        .map_err(|e| format!("Failed to fetch remote SRF: {}", e))?
        .into_json()
        .map_err(|e| format!("Failed to parse remote SRF: {}", e))
}

pub fn scan_local_mods(
    agent: &mut ureq::Agent,
    repo_url: &str,
    base_path: &Path,
    repository: &Repository,
    status_sender: &Sender<CommandMessage>,
) -> Result<Vec<ModUpdate>, String> {
    let mut updates_needed = Vec::new();
    let temp_dir = base_path.join(TEMP_FOLDER);
    
    // Create temp directory if it doesn't exist
    if !temp_dir.exists() {
        fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    }

    for required_mod in &repository.required_mods {
        status_sender.send(CommandMessage::ScanningStatus(
            format!("Scanning {}", required_mod.mod_name)
        )).ok();

        let mod_path = base_path.join(&required_mod.mod_name);
        let remote_mod = download_remote_srf(agent, repo_url, &required_mod.mod_name)?;

        if !mod_path.exists() {
            // Mod doesn't exist locally, collect all files from remote
            let files = remote_mod.files.iter().map(|f| FileUpdate {
                path: f.path.clone(),
                checksum: f.checksum.clone(),
                size: f.length,
            }).collect();

            updates_needed.push(ModUpdate {
                name: required_mod.mod_name.clone(),
                files,
            });
            continue;
        }

        let srf_path = mod_path.join("mod.srf");
        let local_mod = if srf_path.exists() {
            match read_srf_file(&srf_path) {
                Ok(local_mod) => local_mod,
                Err(_) => {
                    // Invalid local SRF, collect all files from remote
                    let files = remote_mod.files.iter().map(|f| FileUpdate {
                        path: f.path.clone(),
                        checksum: f.checksum.clone(),
                        size: f.length,
                    }).collect();

                    updates_needed.push(ModUpdate {
                        name: required_mod.mod_name.clone(),
                        files,
                    });
                    continue;
                }
            }
        } else {
            // No local SRF, collect all files from remote
            let files = remote_mod.files.iter().map(|f| FileUpdate {
                path: f.path.clone(),
                checksum: f.checksum.clone(),
                size: f.length,
            }).collect();

            updates_needed.push(ModUpdate {
                name: required_mod.mod_name.clone(),
                files,
            });
            continue;
        };

        // Compare files between local and remote
        let mut different_files = Vec::new();
        
        for remote_file in &remote_mod.files {
            let local_file = local_mod.files.iter().find(|f| f.path == remote_file.path);
            
            match local_file {
                Some(local_file) => {
                    // Check if file needs updating
                    if local_file.checksum != remote_file.checksum {
                        different_files.push(FileUpdate {
                            path: remote_file.path.clone(),
                            checksum: remote_file.checksum.clone(),
                            size: remote_file.length,
                        });
                    }
                }
                None => {
                    // File doesn't exist locally
                    different_files.push(FileUpdate {
                        path: remote_file.path.clone(),
                        checksum: remote_file.checksum.clone(),
                        size: remote_file.length,
                    });
                }
            }
        }

        if !different_files.is_empty() {
            updates_needed.push(ModUpdate {
                name: required_mod.mod_name.clone(),
                files: different_files,
            });
        }
    }

    // Cleanup temp directory
    if temp_dir.exists() {
        let _ = fs::remove_dir_all(&temp_dir);
    }

    Ok(updates_needed)
}

fn read_srf_file(path: &Path) -> Result<srf::Mod, String> {
    let file = fs::File::open(path).map_err(|e| format!("Failed to open SRF file: {}", e))?;
    let mut reader = io::BufReader::new(file);

    if srf::is_legacy_srf(&mut reader).map_err(|e| format!("Failed to check SRF format: {}", e))? {
        srf::deserialize_legacy_srf(&mut reader).map_err(|e| format!("Failed to parse legacy SRF: {}", e))
    } else {
        serde_json::from_reader(reader).map_err(|e| format!("Failed to parse SRF: {}", e))
    }
}

fn compare_mod_files(
    repo_url: &str,
    mod_name: &str,
    local_mod: &srf::Mod,
    remote_checksum: String,
    agent: &mut ureq::Agent,
) -> Result<Vec<FileUpdate>, String> {
    let mut different_files = Vec::new();

    // Fetch remote mod.srf
    let remote_srf_url = format!("{}/{}/mod.srf", repo_url.trim_end_matches('/'), mod_name);
    let remote_mod: srf::Mod = agent
        .get(&remote_srf_url)
        .call()
        .map_err(|e| format!("Failed to fetch remote SRF: {}", e))?
        .into_json()
        .map_err(|e| format!("Failed to parse remote SRF: {}", e))?;

    // Compare files
    for remote_file in &remote_mod.files {
        if let Some(local_file) = local_mod.files.iter().find(|f| f.path == remote_file.path) {
            if local_file.checksum != remote_file.checksum {
                different_files.push(FileUpdate {
                    path: remote_file.path.clone(),
                    checksum: remote_file.checksum.clone(),
                    size: remote_file.length,
                });
            }
        } else {
            // File doesn't exist locally
            different_files.push(FileUpdate {
                path: remote_file.path.clone(),
                checksum: remote_file.checksum.clone(),
                size: remote_file.length,
            });
        }
    }

    Ok(different_files)
}
