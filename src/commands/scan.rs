use crate::repository::Repository;
use crate::gui::state::CommandMessage;
use crate::srf;
use relative_path::RelativePathBuf;
use std::path::Path;
use std::sync::mpsc::Sender;
use std::{fs, io};
use indicatif::{MultiProgress, ProgressBar, ProgressStyle};

#[derive(Debug, Clone)]
pub struct ModUpdate {
    pub name: String,
    pub files: Vec<FileUpdate>,
}

#[derive(Debug, Clone)]
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
    let base_url = crate::repository::normalize_repo_url(repo_url);
    let remote_srf_url = format!("{}{}/mod.srf", base_url, mod_name);
    
    agent
        .get(&remote_srf_url)
        .call()
        .map_err(|e| format!("Failed to fetch remote SRF: {}", e))?
        .into_json()
        .map_err(|e| format!("Failed to parse remote SRF: {}", e))
}

fn create_file_updates(files: &[srf::File]) -> Vec<FileUpdate> {
    files.iter().map(|f| FileUpdate {
        path: f.path.clone(),
        checksum: f.checksum.clone(),
        size: f.length,
    }).collect()
}

pub fn scan_local_mods(
    agent: &mut ureq::Agent,
    repo_url: &str,
    base_path: &Path,
    repository: &Repository,
    status_sender: &Sender<CommandMessage>,
    force_sync: bool,
) -> Result<Vec<ModUpdate>, String> {
    let required_mods = repository.required_mods.clone();
    let total_mods = required_mods.len();
    
    let multi = MultiProgress::new();
    let overall_progress = multi.add(ProgressBar::new_spinner());
    overall_progress.set_style(
        ProgressStyle::default_spinner()
            .template("{spinner:.green} {prefix:.bold.dim} {msg}")
            .unwrap()
    );
    overall_progress.set_prefix("Scanning:");

    let scan_bar = multi.add(ProgressBar::new(total_mods as u64));
    scan_bar.set_style(
        ProgressStyle::default_bar()
            .template("{spinner:.green} [{elapsed_precise}] [{bar:40.cyan/blue}] {pos}/{len} mods")
            .unwrap()
    );

    let mut updates_needed = Vec::new();
    let temp_dir = base_path.join(TEMP_FOLDER);
    
    // Create temp directory if it doesn't exist
    if !temp_dir.exists() {
        fs::create_dir_all(&temp_dir)
            .map_err(|e| format!("Failed to create temp directory: {}", e))?;
    }

    for required_mod in required_mods {
        let mod_name = required_mod.mod_name.clone();
        let status_message = format!("Scanning {}", mod_name);
        
        overall_progress.set_message(mod_name.clone());
        scan_bar.set_message(status_message.clone());
        
        status_sender.send(CommandMessage::ScanningStatus(status_message)).ok();

        let mod_path = base_path.join(&required_mod.mod_name);
        let remote_mod = download_remote_srf(agent, repo_url, &required_mod.mod_name)?;

        // If force_sync is true or mod doesn't exist, add all files
        if force_sync || !mod_path.exists() {
            updates_needed.push(ModUpdate {
                name: required_mod.mod_name.clone(),
                files: create_file_updates(&remote_mod.files),
            });
            scan_bar.inc(1);
            continue;
        }

        let srf_path = mod_path.join("mod.srf");
        let local_mod = if srf_path.exists() {
            match read_srf_file(&srf_path) {
                Ok(local_mod) => local_mod,
                Err(_) => {
                    updates_needed.push(ModUpdate {
                        name: required_mod.mod_name.clone(),
                        files: create_file_updates(&remote_mod.files),
                    });
                    scan_bar.inc(1);
                    continue;
                }
            }
        } else {
            updates_needed.push(ModUpdate {
                name: required_mod.mod_name.clone(),
                files: create_file_updates(&remote_mod.files),
            });
            scan_bar.inc(1);
            continue;
        };

        // Compare files between local and remote
        let mut different_files = Vec::new();
        
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
                different_files.push(FileUpdate {
                    path: remote_file.path.clone(),
                    checksum: remote_file.checksum.clone(),
                    size: remote_file.length,
                });
            }
        }

        if !different_files.is_empty() {
            updates_needed.push(ModUpdate {
                name: required_mod.mod_name.clone(),
                files: different_files,
            });
        }

        scan_bar.inc(1);
    }

    scan_bar.finish_with_message("Scan complete");
    overall_progress.finish_with_message(format!("Found {} mods needing updates", updates_needed.len()));

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
