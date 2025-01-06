use std::path::{Path, PathBuf};
use std::fs;
use walkdir::WalkDir;
use crate::repository::Repository;
use crate::md5_digest::Md5Digest;
use md5::Digest;

pub fn save_repository(path: &Path, repo: &mut Repository) -> Result<(), String> {
    // Generate SRF files first and collect checksums
    for mod_entry in &mut repo.required_mods {
        let mod_path = path.join(&mod_entry.mod_name);
        if mod_path.exists() {
            match crate::srf::scan_mod(&mod_path) {
                Ok(srf_mod) => {
                    // Write the SRF file
                    let srf_path = mod_path.join("mod.srf");
                    let srf_file = std::fs::File::create(srf_path)
                        .map_err(|e| format!("Failed to create SRF file: {}", e))?;
                    serde_json::to_writer(srf_file, &srf_mod)
                        .map_err(|e| format!("Failed to write SRF file: {}", e))?;
                    
                    mod_entry.checksum = srf_mod.checksum;
                },
                Err(e) => return Err(format!("Failed to generate SRF for {}: {}", mod_entry.mod_name, e)),
            }
        }
    }

    // Calculate overall repository checksum
    let mut hasher = md5::Md5::new();
    for mod_entry in &repo.required_mods {
        hasher.update(mod_entry.checksum.to_string().as_bytes());
    }
    let final_hash = format!("{:X}", hasher.finalize());
    repo.checksum = Md5Digest::new(&final_hash)
        .map_err(|e| format!("Failed to create checksum: {}", e))?;

    // Save repo.json with updated checksums
    super::scanner::save_repo(path, repo)
}

pub fn clean_directory(path: &Path, force_lowercase: bool, filters: &[String]) -> Result<(), String> {
    remove_filtered_files(path, filters)?;
    
    if force_lowercase {
        rename_to_lowercase(path)?;
    }
    
    Ok(())
}

fn remove_filtered_files(path: &Path, filters: &[String]) -> Result<(), String> {
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let name = path.file_name()
            .and_then(|n| n.to_str())
            .unwrap_or("");
            
        if filters.iter().any(|f| name.contains(f)) {
            if path.is_dir() {
                fs::remove_dir_all(path)
                    .map_err(|e| format!("Failed to remove directory '{}': {}", name, e))?;
            } else {
                fs::remove_file(path)
                    .map_err(|e| format!("Failed to remove file '{}': {}", name, e))?;
            }
        }
    }
    Ok(())
}

// Remove remove_git_files function as it's no longer needed

fn rename_to_lowercase(path: &Path) -> Result<(), String> {
    let mut rename_ops: Vec<(PathBuf, PathBuf)> = Vec::new();
    
    for entry in WalkDir::new(path).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        let filename = path.file_name()
            .and_then(|n| n.to_str())
            .ok_or_else(|| "Invalid filename".to_string())?;
            
        let lowercase = filename.to_lowercase();
        if filename != lowercase {
            let new_path = path.with_file_name(lowercase);
            rename_ops.push((path.to_path_buf(), new_path));
        }
    }
    
    for (old_path, new_path) in rename_ops {
        fs::rename(&old_path, &new_path)
            .map_err(|e| format!("Failed to rename '{}': {}", old_path.display(), e))?;
    }
    
    Ok(())
}
