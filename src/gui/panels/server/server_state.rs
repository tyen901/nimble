use eframe::egui;
use std::path::PathBuf;
use crate::repository::Repository;
use crate::gui::widgets::{PathPicker, StatusDisplay, CommandHandler};
use crate::gui::state::{CommandMessage, GuiState, Profile, GuiConfig};
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub struct ServerState {
    pub path_picker: PathPicker,
    pub repository: Option<Repository>,
    pub status: StatusDisplay,
    sync_cancel: Arc<AtomicBool>,
    scan_results: Option<Vec<crate::commands::scan::ModUpdate>>,
    pub profiles: Vec<Profile>,
    pub selected_profile: Option<String>,
    pub editing_profile: Option<Profile>,
    auto_connect: bool,
    first_show: bool,
}

impl Default for ServerState {
    fn default() -> Self {
        Self {
            path_picker: PathPicker::new("Base Path:", "Select Mods Directory"),
            repository: None,
            status: StatusDisplay::default(),
            sync_cancel: Arc::new(AtomicBool::new(false)),
            scan_results: None,
            profiles: Vec::new(),
            selected_profile: None,
            auto_connect: true,
            editing_profile: None,
            first_show: true,
        }
    }
}

impl CommandHandler for ServerState {}

impl ServerState {
    // Change to public and return owned String
    pub fn get_current_url(&self) -> Option<String> {
        self.selected_profile
            .as_ref()
            .and_then(|name| self.profiles.iter().find(|p| &p.name == name))
            .map(|profile| profile.repo_url.clone())
    }

    // Add this helper method
    pub fn should_auto_connect(&mut self) -> bool {
        if self.first_show && self.auto_connect && self.repository.is_none() {
            self.first_show = false;
            true
        } else {
            false
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>, state: &GuiState) {
        self.first_show = false;
        self.status.show(ui);

        // Profile selector and info section
        ui.group(|ui| {
            ui.heading("Profiles");
            ui.horizontal(|ui| {
                let prev_selection = self.selected_profile.clone();
                egui::ComboBox::from_label("")
                    .selected_text(self.selected_profile.as_deref().unwrap_or("Select Profile"))
                    .show_ui(ui, |ui| {
                        for profile in &self.profiles {
                            ui.selectable_value(&mut self.selected_profile, Some(profile.name.clone()), &profile.name);
                        }
                    });

                // Auto-connect checkbox
                ui.checkbox(&mut self.auto_connect, "Auto-connect");

                // Handle profile selection change
                if prev_selection != self.selected_profile {
                    if let Some(sender) = sender {
                        // First disconnect if connected
                        if self.repository.is_some() {
                            sender.send(CommandMessage::Disconnect).ok();
                            self.repository = None;
                        }

                        // Then update path and trigger config save
                        if let Some(name) = &self.selected_profile {
                            if let Some(profile) = self.profiles.iter().find(|p| &p.name == name) {
                                self.path_picker.set_path(&profile.base_path);
                                sender.send(CommandMessage::ConfigChanged).ok();

                                // Auto-connect to new profile if enabled
                                if self.auto_connect {
                                    sender.send(CommandMessage::ConnectionStarted).ok();
                                    crate::gui::panels::server::server_actions::connect_to_server(&profile.repo_url, sender.clone());
                                }
                            }
                        }
                    }
                }

                if ui.button("New").clicked() {
                    self.editing_profile = Some(Profile::default());
                }

                if let Some(selected) = &self.selected_profile {
                    if ui.button("Edit").clicked() {
                        if let Some(profile) = self.profiles.iter().find(|p| p.name == *selected) {
                            self.editing_profile = Some(profile.clone());
                        }
                    }
                    if ui.button("Delete").clicked() {
                        self.profiles.retain(|p| p.name != *selected);
                        self.selected_profile = None;
                        if let Some(sender) = sender {
                            sender.send(CommandMessage::ConfigChanged).ok();
                        }
                    }
                }
            });

            // Show selected profile info
            if let Some(name) = &self.selected_profile {
                if let Some(profile) = self.profiles.iter().find(|p| &p.name == name) {
                    ui.add_space(8.0);
                    ui.label(format!("Name: {}", profile.name));
                    ui.label(format!("URL: {}", profile.repo_url));
                    ui.label(format!("Path: {}", profile.base_path.display()));
                    
                    ui.add_space(8.0);
                    ui.horizontal(|ui| {
                        let is_connected = self.repository.is_some();
                        if is_connected {
                            if ui.button("Disconnect").clicked() && sender.is_some() {
                                sender.unwrap().send(CommandMessage::Disconnect).ok();
                            }
                        } else if !matches!(state, GuiState::Connecting) {
                            if ui.button("Connect").clicked() && sender.is_some() {
                                let sender = sender.unwrap().clone();
                                sender.send(CommandMessage::ConnectionStarted).ok();
                                crate::gui::panels::server::server_actions::connect_to_server(&profile.repo_url, sender);
                            }
                        }
                    });

                    if matches!(state, GuiState::Connecting) {
                        ui.horizontal(|ui| {
                            ui.spinner();
                            ui.label("Connecting to server...");
                        });
                    }
                }
            }
        });

        // Profile editor dialog
        if let Some(editing) = &mut self.editing_profile {
            let mut should_close = false;
            egui::Window::new("Edit Profile")
                .show(ui.ctx(), |ui| {
                    ui.horizontal(|ui| {
                        ui.label("Name:");
                        ui.text_edit_singleline(&mut editing.name);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Repository URL:");
                        ui.text_edit_singleline(&mut editing.repo_url);
                    });
                    ui.horizontal(|ui| {
                        ui.label("Base Path:");
                        let path_str = editing.base_path.to_string_lossy();
                        ui.label(path_str);
                        if ui.button("Browse").clicked() {
                            if let Some(path) = rfd::FileDialog::new().pick_folder() {
                                editing.base_path = path;
                            }
                        }
                    });
                    ui.horizontal(|ui| {
                        if ui.button("Save").clicked() {
                            if !editing.name.is_empty() {
                                // Remove existing profile with same name
                                self.profiles.retain(|p| p.name != editing.name);
                                // Add new/updated profile
                                self.profiles.push(editing.clone());
                                self.selected_profile = Some(editing.name.clone());
                                // Update current settings
                                self.path_picker.set_path(&editing.base_path);
                                if let Some(sender) = sender {
                                    sender.send(CommandMessage::ConfigChanged).ok();
                                }
                                should_close = true;
                            }
                        }
                        if ui.button("Cancel").clicked() {
                            should_close = true;
                        }
                    });
                });
            
            if should_close {
                self.editing_profile = None;
            }
        }

        if self.repository.is_none() {
            return;
        }

        // Show repository UI when connected
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
                // Repository info section
                ui.group(|ui| {
                    ui.horizontal(|ui| {
                        ui.heading(&repo_name);
                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                            let is_active = !matches!(state, GuiState::Syncing { .. } | GuiState::Scanning { .. });
                            if is_active {
                                if ui.button("Disconnect").clicked() && sender.is_some() {
                                    sender.unwrap().send(CommandMessage::Disconnect).ok();
                                }
                            }
                        });
                    });
                    ui.label(format!("Version: {}", version));
                    ui.label(format!("Required Mods: {}", required_mods_count));
                    ui.label(format!("Optional Mods: {}", optional_mods_count));
                });
                ui.add_space(8.0);

                // Path section
                ui.group(|ui| {
                    ui.label("Local Installation Path:");
                    ui.horizontal(|ui| {
                        let path = self.path_picker.path();
                        let path_str = path.to_str().unwrap_or("");
                        ui.horizontal_wrapped(|ui| {
                            ui.label(path_str);
                        });
                        let is_active = !matches!(state, GuiState::Syncing { .. } | GuiState::Scanning { .. });
                        if is_active {
                            if ui.button("📂 Edit").clicked() {
                                if self.path_picker.show_picker() && sender.is_some() {
                                    sender.unwrap().send(CommandMessage::ConfigChanged).ok();
                                }
                            }
                        }
                    });
                });
                ui.add_space(8.0);

                // Status/Progress section
                match state {
                    GuiState::Scanning { message } => {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label(message);
                            });
                        });
                    }
                    GuiState::Syncing { progress, current_file, files_processed, total_files } => {
                        ui.group(|ui| {
                            ui.heading("Sync Progress");
                            ui.label(format!("Files: {} / {}", files_processed, total_files));
                            ui.label(format!("Current: {}", current_file));
                            ui.add(egui::ProgressBar::new(*progress).show_percentage());
                            
                            if ui.button("Stop").clicked() {
                                // Use SeqCst ordering for immediate visibility
                                self.sync_cancel.store(true, Ordering::SeqCst);
                                if let Some(sender) = sender {
                                    sender.send(CommandMessage::CancelSync).ok();
                                }
                            }
                        });
                        self.scan_results = None;
                    }
                    _ => {
                        // Action buttons
                        ui.horizontal(|ui| {
                            self.show_scan_button(ui, sender);
                            ui.add_space(8.0);
                            self.show_sync_button(ui, sender);
                            ui.add_space(8.0);
                            self.show_launch_button(ui, sender);
                        });
                        // Status messages
                        self.status.show(ui);

                        // Show scan results if available
                        if let Some(scan_results) = &self.scan_results {
                            ui.group(|ui| {
                                ui.heading("Scan Results");
                                for mod_update in scan_results {
                                    ui.label(format!("Mod: {}", mod_update.name));
                                    for file_update in &mod_update.files {
                                        ui.label(format!("  File: {}", file_update.path));
                                    }
                                }
                            });
                        }
                    }
                }
            });
        }
    }

    fn show_sync_button(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>) {
        if ui.button("Sync Mods").clicked() {
            let Some(repo_url) = self.get_current_url() else {
                self.status.set_error("No profile selected");
                return;
            };

            let base_path = self.path_picker.path();
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
                self.sync_cancel.store(false, Ordering::SeqCst);
                self.scan_results = None;
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
                let sender_clone = sender.clone();
                std::thread::spawn(move || {
                    if let Err(e) = crate::commands::launch::launch(&base_path) {
                        sender_clone.send(CommandMessage::LaunchError(e.to_string())).ok();
                    } else {
                        sender_clone.send(CommandMessage::LaunchComplete).ok();
                    }
                });
            }
        }
    }

    fn show_scan_button(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>) {
        if ui.button("Scan Mods").clicked() {
            let Some(repo_url) = self.get_current_url() else {
                self.status.set_error("No profile selected");
                return;
            };

            let base_path = self.path_picker.path();
            
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
                let repo = self.repository.as_ref().unwrap().clone();
                let repo_url = self.get_current_url().unwrap();
                let base_path = base_path.clone();
                let sender_clone = sender.clone();
                
                sender.send(CommandMessage::ScanStarted).ok();
                
                std::thread::spawn(move || {
                    let mut agent = ureq::agent();
                    match crate::commands::scan::scan_local_mods(
                        &mut agent,
                        &repo_url,
                        &base_path,
                        &repo,
                        &sender_clone
                    ) {
                        Ok(updates) => {
                            let total_files: usize = updates.iter()
                                .map(|m| m.files.len().max(1))
                                .sum();
                            
                            if updates.is_empty() {
                                sender_clone.send(CommandMessage::ScanningStatus(
                                    "All mods are up to date".into()
                                )).ok();
                            } else {
                                let msg = format!(
                                    "Found {} mod(s) that need updating ({} files)",
                                    updates.len(),
                                    total_files
                                );
                                sender_clone.send(CommandMessage::ScanningStatus(msg)).ok();
                            }
                            std::thread::sleep(std::time::Duration::from_secs(2));
                            sender_clone.send(CommandMessage::SyncComplete).ok();
                        }
                        Err(e) => {
                            sender_clone.send(CommandMessage::SyncError(e)).ok();
                        }
                    }
                });
            }
        }
    }

    pub fn set_repository(&mut self, repo: Repository) {
        self.repository = Some(repo);
    }

    pub fn repository(&self) -> Option<&Repository> {
        self.repository.as_ref()
    }

    fn start_sync_with_context(base_path: PathBuf, repo_url: &str, sync_cancel: Arc<AtomicBool>, sender: Sender<CommandMessage>) {
        let repo_url = repo_url.to_string();
        let context = crate::commands::sync::SyncContext {
            cancel: sync_cancel,
            status_sender: Some(sender.clone()),
        };
        
        std::thread::spawn(move || {
            let mut agent = ureq::agent();
            match crate::commands::sync::sync_with_context(&mut agent, &repo_url, &base_path, false,false, &context) {
                Ok(()) => sender.send(CommandMessage::SyncComplete),
                Err(crate::commands::sync::Error::Cancelled) => sender.send(CommandMessage::SyncCancelled),
                Err(e) => sender.send(CommandMessage::SyncError(e.to_string())),
            }.ok();
        });
    }

    pub fn load_from_config(&mut self, config: &GuiConfig) {
        self.profiles = config.profiles.clone();
        self.selected_profile = config.selected_profile.clone();
        if let Some(profile) = config.get_selected_profile() {
            self.path_picker.set_path(&profile.base_path);
        }
    }

    pub fn save_to_config(&self, config: &mut GuiConfig) {
        config.profiles = self.profiles.clone();
        config.selected_profile = self.selected_profile.clone();
    }
}