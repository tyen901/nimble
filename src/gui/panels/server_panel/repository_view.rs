use eframe::egui;
use crate::repository::Repository;
use crate::gui::widgets::PathPicker;
use crate::gui::state::CommandMessage;
use std::sync::mpsc::Sender;

pub struct RepositoryView {
    pub path_picker: PathPicker,
    repository: Option<Repository>,
    repo_url: String,
    error: Option<String>,
}

impl Default for RepositoryView {
    fn default() -> Self {
        Self {
            path_picker: PathPicker::new("Base Path:", "Select Mods Directory"),
            repository: None,
            repo_url: String::new(),
            error: None,
        }
    }
}

impl RepositoryView {
    pub fn show(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>) {
        if let Some(repo) = &self.repository {
            ui.heading(&repo.repo_name);
            ui.label(format!("Version: {}", repo.version));
            ui.label(format!("Required Mods: {}", repo.required_mods.len()));
            ui.label(format!("Optional Mods: {}", repo.optional_mods.len()));
            ui.add_space(8.0);

            self.path_picker.show(ui);
            
            if let Some(error) = &self.error {
                ui.colored_label(ui.style().visuals.error_fg_color, error);
            }

            ui.horizontal(|ui| {
                if ui.button("Sync Mods").clicked() {
                    self.handle_sync(sender);
                }
                
                if ui.button("Launch Game").clicked() {
                    self.handle_launch(sender);
                }
            });
        }
    }

    pub fn set_repository(&mut self, repo: Repository, url: String) {
        self.repository = Some(repo);
        self.repo_url = url;
    }

    pub fn repository(&self) -> Option<&Repository> {
        self.repository.as_ref()
    }

    fn handle_sync(&mut self, sender: Option<&Sender<CommandMessage>>) {
        self.error = None;
        if let Err(e) = self.validate_paths() {
            self.error = Some(e);
        } else if let Some(sender) = sender {
            self.start_sync(sender.clone());
        }
    }

    fn handle_launch(&mut self, sender: Option<&Sender<CommandMessage>>) {
        self.error = None;
        if let Err(e) = self.validate_paths() {
            self.error = Some(e);
        } else if let Some(sender) = sender {
            sender.send(CommandMessage::LaunchStarted).ok();
        }
    }

    fn validate_paths(&self) -> Result<(), String> {
        if self.path_picker.path().to_str().unwrap_or("").trim().is_empty() {
            return Err("Base path is required".into());
        }
        Ok(())
    }

    fn start_sync(&self, sender: Sender<CommandMessage>) {
        let base_path = self.path_picker.path();
        let repo_url = self.repo_url.clone();
        std::thread::spawn(move || {
            let mut agent = ureq::agent();
            match crate::commands::sync::sync(&mut agent, &repo_url, &base_path, false) {
                Ok(()) => sender.send(CommandMessage::SyncComplete),
                Err(e) => sender.send(CommandMessage::SyncError(e.to_string())),
            }.ok();
        });
    }
}
