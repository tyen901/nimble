mod connection_view;
mod repository_view;

use eframe::egui;
use crate::gui::state::{GuiState, GuiConfig, CommandMessage};
use crate::repository::Repository;
use std::sync::mpsc::Sender;
use connection_view::ConnectionView;
use repository_view::RepositoryView;

pub struct ServerPanel {
    connection_view: ConnectionView,
    repository_view: RepositoryView,
}

impl Default for ServerPanel {
    fn default() -> Self {
        Self {
            connection_view: ConnectionView::default(),
            repository_view: RepositoryView::default(),
        }
    }
}

impl ServerPanel {
    pub fn from_config(config: &GuiConfig) -> Self {
        let mut panel = Self::default();
        
        // Store URL in both views to maintain state
        panel.connection_view.repo_url = config.repo_url.clone();
        panel.repository_view.set_url(config.repo_url.clone());
        
        if config.base_path.exists() {
            panel.repository_view.path_picker.set_path(&config.base_path);
        }
        panel
    }

    pub fn repo_url(&self) -> &str {
        &self.connection_view.repo_url
    }

    pub fn base_path(&self) -> std::path::PathBuf {
        self.repository_view.path_picker.path()
    }

    pub fn show(&mut self, ui: &mut egui::Ui, state: &GuiState, sender: Option<&Sender<CommandMessage>>) {
        ui.heading("Server Connection");
        ui.add_space(8.0);

        match state {
            GuiState::Idle => {
                if self.repository_view.repository().is_none() {
                    self.connection_view.show(ui, sender, state);
                } else {
                    self.repository_view.show(ui, sender, state);
                }
            },
            GuiState::Connecting => {
                ui.spinner();
                ui.label("Connecting to server...");
            },
            GuiState::Syncing { .. } | GuiState::Scanning { .. } => {
                // Show repository view first
                self.repository_view.show(ui, sender, state);
            },
            GuiState::Launching => {
                ui.spinner();
                ui.label("Launching game...");
            },
            _ => {}
        }
    }

    pub fn set_repository(&mut self, repo: Repository) {
        // Pass both repository and URL when setting up repository view
        self.repository_view.set_repository(repo, self.connection_view.repo_url.clone());
    }

    pub fn handle_command(&mut self, command: &CommandMessage) {
        match command {
            CommandMessage::Disconnect => {
                self.repository_view = RepositoryView::default();
            }
            CommandMessage::ScanStarted => {
                // Handle scan started command
                self.repository_view.status.set_info("Scanning local folder...");
                // Implement the scanning logic here
            }
            _ => {}
        }
    }
}
