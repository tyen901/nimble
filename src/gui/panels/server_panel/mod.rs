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
        panel.connection_view.repo_url = config.repo_url.clone();
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
                    self.connection_view.show(ui, sender);
                } else {
                    self.repository_view.show(ui, sender, state);
                }
            },
            GuiState::Connecting => {
                ui.spinner();
                ui.label("Connecting to server...");
            },
            GuiState::Syncing { progress, current_file, files_processed, total_files } => {
                ui.label(format!("Syncing: {} / {} files", files_processed, total_files));
                ui.label(format!("Current file: {}", current_file));
                ui.add(egui::ProgressBar::new(*progress).show_percentage().animate(true));
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
        self.repository_view.set_repository(repo, self.connection_view.repo_url.clone());
    }
}
