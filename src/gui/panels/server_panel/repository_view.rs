use eframe::egui;
use crate::repository::Repository;
use crate::gui::widgets::PathPicker;
use crate::gui::state::{CommandMessage, GuiState};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct RepositoryView {
    pub path_picker: PathPicker,
    repository: Option<Repository>,
    repo_url: String,
    error: Option<String>,
    sync_cancel: Arc<AtomicBool>,
}

impl Default for RepositoryView {
    fn default() -> Self {
        Self {
            path_picker: PathPicker::new("Base Path:", "Select Mods Directory"),
            repository: None,
            repo_url: String::new(),
            error: None,
            sync_cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl RepositoryView {
    pub fn show(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>, state: &GuiState) {
        if let Some(repo) = &self.repository {
            ui.heading(&repo.repo_name);
            ui.label(format!("Version: {}", repo.version));
            ui.label(format!("Required Mods: {}", repo.required_mods.len()));
            ui.label(format!("Optional Mods: {}", repo.optional_mods.len()));
            ui.add_space(8.0);

            match state {
                GuiState::Syncing { .. } => {
                    if let Some(sender) = sender {
                        if ui.button("Stop Sync").clicked() {
                            self.sync_cancel.store(true, Ordering::Relaxed);
                            sender.send(CommandMessage::CancelSync).ok();
                        }
                    }
                },
                _ => {
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
        let context = crate::commands::sync::SyncContext {
            cancel: self.sync_cancel.clone(),
        };
        
        std::thread::spawn(move || {
            let mut agent = ureq::agent();
            match crate::commands::sync::sync_with_context(&mut agent, &repo_url, &base_path, false, &context) {
                Ok(()) => sender.send(CommandMessage::SyncComplete),
                Err(crate::commands::sync::Error::Cancelled) => sender.send(CommandMessage::SyncCancelled),
                Err(e) => sender.send(CommandMessage::SyncError(e.to_string())),
            }.ok();
        });
    }
}
