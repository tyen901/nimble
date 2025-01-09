use eframe::egui;
use std::path::{Path, PathBuf};
use super::{state::CreateRepoPanelState, scanner, ui};

pub struct CreateRepoPanel {
    state: CreateRepoPanelState,
}

impl Default for CreateRepoPanel {
    fn default() -> Self {
        Self {
            state: CreateRepoPanelState::default(),
        }
    }
}

impl CreateRepoPanel {

    fn scan_mods(&mut self, path: &PathBuf) {
        if self.state.last_scanned_path.as_ref() == Some(path) {
            return;
        }

        let found_mods = scanner::scan_directory(path);

        match scanner::load_existing_repo(path) {
            Ok(mut loaded_repo) => {
                loaded_repo.required_mods = found_mods;
                self.state.repo = loaded_repo;
                self.state.status.set_info("Updated repository with current mods");
            },
            Err(_) => {
                scanner::update_mods_list(&mut self.state.repo, found_mods);
                self.state.status.set_info(format!("Found {} mods", self.state.repo.required_mods.len()));
            }
        }

        self.state.last_scanned_path = Some(path.clone());
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let prev_path = self.state.base_path.path().to_path_buf();
        ui::render_panel(ui, &mut self.state);
        
        let current_path = self.state.base_path.path();
        if current_path != prev_path && !current_path.as_os_str().is_empty() && current_path.exists() {
            self.scan_mods(&current_path);
        }
    }
}
