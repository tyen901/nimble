use eframe::egui;
use std::path::{Path, PathBuf};
use crate::gui::state::GuiConfig;
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
    pub fn from_config(config: &GuiConfig) -> Self {
        let mut panel = Self::default();
        panel.state.config = Some(config.clone());

        // Try to use the path from the selected profile, if any
        if let Some(profile) = config.get_selected_profile() {
            let path = &profile.base_path;
            if path.exists() {
                panel.state.base_path.set_path(path);
                panel.scan_mods(path);
            }
        }

        panel
    }

    fn update_config(&mut self) {
        // No longer need to update last_repo_path since we're using profiles
        // Config updates should be handled through the server panel
    }

    fn scan_mods(&mut self, path: &PathBuf) {
        if self.state.last_scanned_path.as_ref() == Some(path) {
            return;
        }

        // Scan for mods in directory first
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

    pub fn get_current_path(&self) -> PathBuf {
        self.state.base_path.path().to_path_buf()
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        let prev_path = self.state.base_path.path().to_path_buf();
        ui::render_panel(ui, &mut self.state);
        
        let current_path = self.state.base_path.path();
        if current_path != prev_path && !current_path.as_os_str().is_empty() && current_path.exists() {
            self.scan_mods(&current_path);
            self.update_config();
        }
    }
}
