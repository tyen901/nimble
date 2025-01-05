use eframe::egui;
use std::path::PathBuf;
use crate::repository::Repository;
use crate::gui::widgets::{PathPicker, StatusDisplay, CommandHandler};
use crate::gui::state::{CommandMessage, GuiState};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct RepositoryView {
    pub path_picker: PathPicker,
    repository: Option<Repository>,
    repo_url: String,
    status: StatusDisplay,
    sync_cancel: Arc<AtomicBool>,
}

impl Default for RepositoryView {
    fn default() -> Self {
        Self {
            path_picker: PathPicker::new("Base Path:", "Select Mods Directory"),
            repository: None,
            repo_url: String::new(),
            status: StatusDisplay::default(),
            sync_cancel: Arc::new(AtomicBool::new(false)),
        }
    }
}

impl CommandHandler for RepositoryView {}

impl RepositoryView {
    pub fn show(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>, state: &GuiState) {
        // Extract repository data outside the closure
        let repo_data = self.repository.as_ref().map(|repo| {
            (
                repo.repo_name.clone(),
                repo.version.clone(),
                repo.required_mods.len(),
                repo.optional_mods.len(),
            )
        });

        if let Some((repo_name, version, required_mods_count, optional_mods_count)) = repo_data {
            ui.vertical(|ui| {
                // Repository header section
                ui.heading(&repo_name);
                ui.add_space(4.0);
                
                // Repository info section
                ui.group(|ui| {
                    ui.label(format!("Version: {}", version));
                    ui.label(format!("Required Mods: {}", required_mods_count));
                    ui.label(format!("Optional Mods: {}", optional_mods_count));
                });
                ui.add_space(8.0);

                // Path section with edit button
                ui.group(|ui| {
                    ui.label("Local Installation Path:");
                    ui.horizontal(|ui| {
                        let path = self.path_picker.path();
                        let path_str = path.to_str().unwrap_or("");
                        ui.horizontal_wrapped(|ui| {
                            ui.label(path_str);
                        });
                        if ui.button("📂 Edit").clicked() {
                            if self.path_picker.show_picker() && sender.is_some() {
                                sender.unwrap().send(CommandMessage::ConfigChanged).ok();
                            }
                        }
                    });
                });
                ui.add_space(4.0);
            });

            // Handle state outside vertical layout to avoid borrow issues
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
                    self.status.show(ui);
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        self.show_sync_button(ui, sender);
                        ui.add_space(8.0);
                        self.show_launch_button(ui, sender);
                    });
                }
            }
        }
    }

    fn show_sync_button(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>) {
        if ui.button("Sync Mods").clicked() {
            // Extract all values before any validation or status updates
            let base_path = self.path_picker.path();
            let repo_url = self.repo_url.clone();
            let sync_cancel = self.sync_cancel.clone();
            
            // Validate repository exists
            if self.repository.is_none() {
                self.status.set_error("No repository connected");
                return;
            }
            
            if base_path.to_str().unwrap_or("").trim().is_empty() {
                self.status.set_error("Base path is required");
                return;
            }
            
            if let Some(sender) = sender {
                self.sync_cancel.store(false, Ordering::Relaxed); // Reset cancel flag
                Self::start_sync_with_context(base_path, &repo_url, sync_cancel, sender.clone());
            }
        }
    }

    fn show_launch_button(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>) {
        if ui.button("Launch Game").clicked() {
            // Extract path before validation
            let base_path = self.path_picker.path();
            
            if base_path.to_str().unwrap_or("").trim().is_empty() {
                self.status.set_error("Base path is required");
                return;
            }
            
            if let Some(sender) = sender {
                sender.send(CommandMessage::LaunchStarted).ok();
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

    fn start_sync_with_context(base_path: PathBuf, repo_url: &str, sync_cancel: Arc<AtomicBool>, sender: Sender<CommandMessage>) {
        let repo_url = repo_url.to_string();
        let context = crate::commands::sync::SyncContext {
            cancel: sync_cancel,
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
