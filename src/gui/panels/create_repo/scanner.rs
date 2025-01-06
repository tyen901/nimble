use std::path::{Path, PathBuf};
use std::fs;
use walkdir::WalkDir;
use semver::Version;
use crate::repository::{Repository, Mod};
use crate::md5_digest::Md5Digest;

pub fn scan_directory(path: &Path) -> Vec<Mod> {
    WalkDir::new(path)
        .min_depth(1)
        .max_depth(1)
        .into_iter()
        .filter_map(Result::ok)
        .filter(|e| e.file_type().is_dir() && e.file_name().to_string_lossy().starts_with('@'))
        .map(|entry| Mod {
            mod_name: entry.file_name().to_string_lossy().to_string(),
            checksum: Md5Digest::default(),
            enabled: true,
        })
        .collect()
}

pub fn load_existing_repo(path: &Path) -> Result<Repository, String> {
    let repo_file = path.join("repo.json");
    fs::read_to_string(&repo_file)
        .map_err(|e| format!("Failed to read repo.json: {}", e))
        .and_then(|contents| {
            serde_json::from_str(&contents)
                .map_err(|e| format!("Failed to parse repo.json: {}", e))
        })
}

pub fn update_mods_list(repo: &mut Repository, new_mods: Vec<Mod>) {  // Removed auto_increment parameter
    repo.required_mods = new_mods;
}

pub fn save_repo(path: &Path, repo: &Repository) -> Result<(), String> {
    std::fs::File::create(path.join("repo.json"))
        .map_err(|e| format!("Failed to create repo.json: {}", e))
        .and_then(|file| {
            serde_json::to_writer_pretty(file, repo)
                .map_err(|e| format!("Failed to write repo.json: {}", e))
        })
}

pub fn check_for_changes(current_mods: &[Mod], new_mods: &[Mod]) -> bool {
    new_mods.len() != current_mods.len() || 
    new_mods.iter().any(|m| !current_mods.iter().any(|rm| rm.mod_name == m.mod_name))
}
