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

        if let Some(path) = config.last_repo_path() {
            if path.exists() {
                panel.state.base_path.set_path(path);
                panel.scan_mods(&path);
            }
        }

        panel
    }

    fn update_config(&mut self) {
        if let Some(config) = &mut self.state.config {
            config.set_last_repo_path(Some(self.state.base_path.path().to_path_buf()));
        }
    }

    fn scan_mods(&mut self, path: &PathBuf) {
        if self.state.last_scanned_path.as_ref() == Some(path) {
            return;
        }

        match scanner::load_existing_repo(path) {
            Ok(loaded_repo) => {
                self.state.repo = loaded_repo;
                self.state.status.set_info("Loaded existing repo.json");
                self.scan_for_changes(path);
            },
            Err(_) => {
                let new_mods = scanner::scan_directory(path);
                scanner::update_mods_list(&mut self.state.repo, new_mods, self.state.auto_increment_version);
                self.state.status.set_info(format!("Found {} mods", self.state.repo.required_mods.len()));
            }
        }

        self.state.last_scanned_path = Some(path.clone());
    }

    fn scan_for_changes(&mut self, path: &PathBuf) {
        let new_mods = scanner::scan_directory(path);
        if scanner::check_for_changes(&self.state.repo.required_mods, &new_mods) {
            self.state.pending_mods = Some(new_mods);
            self.state.show_update_prompt = true;
            self.state.status.set_info("Found changes in mods. Choose whether to update.");
        }
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
