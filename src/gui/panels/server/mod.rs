mod server_state;
mod server_actions;

use eframe::egui;
use crate::gui::state::{GuiState, GuiConfig, CommandMessage};
use crate::repository::Repository;
use std::sync::mpsc::Sender;
use crate::gui::panels::server::server_state::ServerState;

pub struct ServerPanel {
    state: ServerState,
}

impl Default for ServerPanel {
    fn default() -> Self {
        Self {
            state: ServerState::default(),
        }
    }
}

impl ServerPanel {
    pub fn from_config(config: &GuiConfig) -> Self {
        let mut panel = Self::default();
        panel.state.load_from_config(config);
        if let Err(e) = config.validate() {
            panel.state.status.set_error(e);
        }
        panel
    }

    pub fn save_to_config(&self, config: &mut GuiConfig) {
        self.state.save_to_config(config);
    }

    pub fn base_path(&self) -> std::path::PathBuf {
        self.state.path_picker.path()
    }

    pub fn show(&mut self, ui: &mut egui::Ui, state: &GuiState, sender: Option<&Sender<CommandMessage>>) {
        // Check if we should auto-connect on first show
        if self.state.should_auto_connect() {
            if let Some(sender) = sender {
                if let Some(url) = self.state.get_current_url() {
                    sender.send(CommandMessage::ConnectionStarted).ok();
                    crate::gui::panels::server::server_actions::connect_to_server(&url, sender.clone());
                }
            }
        }

        ui.heading("Server Connection");
        ui.add_space(8.0);
        self.state.show(ui, sender, state);
    }

    pub fn set_repository(&mut self, repo: Repository) {
        // Don't pass URL anymore since it's managed through profiles
        self.state.set_repository(repo);
    }

    pub fn handle_command(&mut self, command: &CommandMessage) {
        match command {
            CommandMessage::Disconnect => {
                // Keep profiles but reset repository
                self.state.repository = None;
            }
            CommandMessage::ScanStarted => {
                self.state.status.set_info("Scanning local folder...");
            }
            _ => {}
        }
    }
}
