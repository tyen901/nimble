use std::path::{Path, PathBuf};
use tokio::fs;
use walkdir::WalkDir;
use indicatif::{ProgressBar, ProgressStyle};
use crate::repository::{Repository, Mod};
use crate::md5_digest::Md5Digest;
use tokio::runtime::Runtime;

// Create Runtime once for all async operations
fn get_runtime() -> Runtime {
    Runtime::new().expect("Failed to create Tokio runtime")
}

pub fn scan_directory(path: &Path) -> Vec<Mod> {
    let rt = get_runtime();
    let pb = ProgressBar::new_spinner();
    pb.set_style(ProgressStyle::default_spinner()
        .template("{spinner:.green} {msg}")
        .unwrap());
    pb.set_message("Scanning for mods...");
    
    // Use walkdir directly - it's already efficient
    let mut mods: Vec<Mod> = WalkDir::new(path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir() && e.file_name().to_string_lossy().starts_with('@'))
        .map(|entry| {
            pb.set_message(format!("Found: {}", entry.file_name().to_string_lossy()));
            Mod {
                mod_name: entry.file_name().to_string_lossy().to_string(),
                checksum: Md5Digest::default(),
                enabled: true,
            }
        })
        .collect();

    pb.finish_with_message(format!("Found {} mods", mods.len()));
    mods.sort_by(|a, b| a.mod_name.cmp(&b.mod_name));
    mods
}

pub fn load_existing_repo(path: &Path) -> Result<Repository, String> {
    let rt = get_runtime();

    let repo_file = path.join("repo.json");
    
    // Verify path exists
    if !rt.block_on(async { fs::metadata(&repo_file).await.is_ok() }) {
        return Err("repo.json not found".to_string());
    }

    // Read and parse repo file
    let contents = std::fs::read_to_string(&repo_file)
        .map_err(|e| format!("Failed to read repo.json: {}", e))?;

    serde_json::from_str(&contents)
        .map_err(|e| format!("Failed to parse repo.json: {}", e))
}

pub fn update_mods_list(repo: &mut Repository, new_mods: Vec<Mod>) {  // Removed auto_increment parameter
    repo.required_mods = new_mods;
}

pub fn save_repo(path: &Path, repo: &Repository) -> Result<(), String> {
    let rt = get_runtime();

    // Ensure directory exists
    rt.block_on(async {
        fs::create_dir_all(path).await
            .map_err(|e| format!("Failed to create directory: {}", e))?;

        let repo_path = path.join("repo.json");
        let json = serde_json::to_string_pretty(repo)
            .map_err(|e| format!("Failed to serialize repository: {}", e))?;

        std::fs::write(&repo_path, json) // Changed to std::fs
            .map_err(|e| format!("Failed to write repo.json: {}", e))
    })
}

pub fn check_for_changes(current_mods: &[Mod], new_mods: &[Mod]) -> bool {
    new_mods.len() != current_mods.len() || 
    new_mods.iter().any(|m| !current_mods.iter().any(|rm| rm.mod_name == m.mod_name))
}