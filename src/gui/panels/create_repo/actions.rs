use std::path::Path;
use crate::repository::Repository;

pub fn save_repository(path: &Path, repo: &mut Repository) -> Result<(), String> {
    // Save repo.json
    super::scanner::save_repo(path, repo)?;
    
    // Generate SRF files for each mod
    for mod_entry in &repo.required_mods {
        let mod_path = path.join(&mod_entry.mod_name);
        if mod_path.exists() {
            match crate::srf::scan_mod(&mod_path) {
                Ok(_) => {}, // scan_mod automatically creates mod.srf
                Err(e) => return Err(format!("Failed to generate SRF for {}: {}", mod_entry.mod_name, e)),
            }
        }
    }

    Ok(())
}
