use std::path::Path;
use crate::repository::Repository;
use crate::srf;

pub fn save_repository(path: &Path, repo: &mut Repository, generate_srf: bool) -> Result<(), String> {
    if generate_srf {
        generate_srf_files(path)?;
        update_mod_checksums(path, repo)?;
    }
    
    // Compute final repository checksum before saving
    repo.compute_checksum();
    
    super::scanner::save_repo(path, repo)
}

fn generate_srf_files(path: &Path) -> Result<(), String> {
    crate::commands::gen_srf::gen_srf(
        path,
        Some(path),
        Some(Box::new(|current_mod, _, _, _| {
            println!("Generating SRF for {}", current_mod);
        }))
    ).map_err(|e| e.to_string())
}

fn update_mod_checksums(path: &Path, repo: &mut Repository) -> Result<(), String> {
    for mod_entry in &mut repo.required_mods {
        let mod_path = path.join(&mod_entry.mod_name);
        if mod_path.exists() {
            match srf::scan_mod(&mod_path) {
                Ok(srf_mod) => {
                    mod_entry.checksum = srf_mod.checksum;
                }
                Err(e) => return Err(format!("Failed to scan mod {}: {}", mod_entry.mod_name, e)),
            }
        }
    }
    Ok(())
}
