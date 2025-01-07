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
        if let Err(e) = config.validate() {
            panel.state.status.set_error(e);
        } else {
            panel.state.set_url(config.repo_url.clone());
        }
        
        if config.base_path.exists() {
            panel.state.path_picker.set_path(&config.base_path);
        }
        panel
    }

    pub fn repo_url(&self) -> &str {
        &self.state.repo_url
    }

    pub fn base_path(&self) -> std::path::PathBuf {
        self.state.path_picker.path()
    }

    pub fn show(&mut self, ui: &mut egui::Ui, state: &GuiState, sender: Option<&Sender<CommandMessage>>) {
        ui.heading("Server Connection");
        ui.add_space(8.0);
        self.state.show(ui, sender, state);
    }

    pub fn set_repository(&mut self, repo: Repository) {
        self.state.set_repository(repo, self.state.repo_url.clone());
    }

    pub fn handle_command(&mut self, command: &CommandMessage) {
        match command {
            CommandMessage::Disconnect => {
                self.state = ServerState::default();
            }
            CommandMessage::ScanStarted => {
                self.state.status.set_info("Scanning local folder...");
            }
            _ => {}
        }
    }
}
