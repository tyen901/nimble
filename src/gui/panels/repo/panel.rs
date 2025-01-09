use eframe::egui;
use crate::gui::state::{GuiState, GuiConfig, CommandMessage};
use crate::mod_cache::ModCache;
use crate::repository::Repository;
use std::sync::mpsc::Sender;
use super::state::RepoPanelState;
use super::ui::{
    RepositoryInfoView, 
    ConnectionStatusView,
    LocalInfoView,
    OperationsView
};

pub struct RepoPanel {
    state: RepoPanelState,
}

impl Default for RepoPanel {
    fn default() -> Self {
        Self {
            state: RepoPanelState::default(),
        }
    }
}

impl RepoPanel {
    pub fn from_config(config: &GuiConfig) -> Self {
        let mut panel = Self::default();
        panel.state.profile_manager().load_from_config(config);
        
        // Load first profile if none selected
        if panel.state.profile_manager().get_selected_profile().is_none() {
            if let Some(first_profile) = panel.state.profile_manager().get_first_profile_name() {
                panel.state.set_selected_profile(Some(first_profile));
            }
        } else {
            // Load cache for existing selected profile
            if let Some(profile) = panel.state.profile_manager().get_selected_profile() {
                if let Ok(cache) = ModCache::from_disk_or_empty(&profile.base_path) {
                    panel.state.load_cache(&cache);
                }
            }
        }

        if let Err(e) = config.validate() {
            panel.state.status().set_error(e);
        }
        panel
    }

    pub fn save_to_config(&mut self, config: &mut GuiConfig) {
        self.state.profile_manager().save_to_config(config);
    }

    pub fn show(&mut self, ui: &mut egui::Ui, gui_state: &GuiState, sender: Option<&Sender<CommandMessage>>) {
        // Show status
        self.state.status.show(ui);

        // Profile management
        self.show_profile_management(ui, sender);

        ui.add_space(8.0);

        // Local info
        if self.state.has_local_data() {
            ui.add_space(8.0);
            LocalInfoView::show(ui, &mut self.state);
        }

        // Connection status
        ConnectionStatusView::show(ui, &mut self.state, sender);

        // Remote operations
        if self.state.is_connected() {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            
            OperationsView::show(ui, &mut self.state, gui_state, sender);
        }

        // Launch button
        if self.state.has_local_data() {
            ui.add_space(8.0);
            let base_path = self.state.profile_manager.get_base_path();
            super::actions::show_launch_button(ui, &mut self.state, sender, &base_path);
        }
    }

    pub fn base_path(&mut self) -> std::path::PathBuf {
        self.state.profile_manager().get_base_path()
    }

    pub fn set_repository(&mut self, repo: Repository) {
        self.state.set_repository(repo);
    }

    pub fn handle_command(&mut self, command: &CommandMessage) {
        match command {
            CommandMessage::ConnectionStarted => {
                self.state.set_connecting();
            },
            CommandMessage::ConnectionComplete(repo) => {
                self.state.set_connected(repo.clone());
            },
            CommandMessage::ConnectionError(error) => {
                self.state.set_connection_error(error.clone());
            },
            CommandMessage::Disconnect => {
                self.state.disconnect();
            },
            CommandMessage::ScanStarted => {
                self.state.set_scanning();
            },
            CommandMessage::ScanComplete(_) => {
                self.state.set_idle();
            },
            CommandMessage::SyncStarted => {
                self.state.set_syncing();
            },
            CommandMessage::SyncComplete => {
                self.state.set_idle();
            },
            CommandMessage::LaunchStarted => {
                self.state.set_launching();
            },
            CommandMessage::LaunchComplete => {
                self.state.set_idle();
            },
            _ => {}
        }
    }

    fn show_profile_management(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>) {
        let (changed, selected_profile) = self.state.profile_manager.show_editor(ui, sender);
        if changed {
            if let Some(profile_name) = selected_profile {
                self.state.set_selected_profile(Some(profile_name));
            }
            if let Some(sender) = sender {
                sender.send(CommandMessage::ConfigChanged).ok();
            }
        }
    }
}