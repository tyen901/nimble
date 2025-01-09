use eframe::egui;
use crate::gui::state::{GuiState, GuiConfig, CommandMessage};
use crate::repository::Repository;
use std::sync::mpsc::Sender;
use std::sync::atomic::Ordering;
use super::state::{RepoPanelState, ConnectionState};
use super::connection::{connect_to_server, disconnect};
use super::actions::show_action_buttons;

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
        if let Err(e) = config.validate() {
            panel.state.status().set_error(e);
        }
        panel
    }

    pub fn save_to_config(&mut self, config: &mut GuiConfig) {
        self.state.profile_manager().save_to_config(config);
    }

    pub fn base_path(&mut self) -> std::path::PathBuf {
        self.state.profile_manager().get_base_path()
    }

    fn show_server_info(&mut self, ui: &mut egui::Ui) {
        // Clone the repository data we need
        let repo_info = self.state.repository().cloned();
        
        if let Some(repo) = repo_info {
            ui.group(|ui| {
                ui.heading("Repository Information");
                ui.add_space(4.0);
                
                // Local path display and explorer button in a separate group
                if let Some(profile) = self.state.profile_manager.get_selected_profile() {
                    let base_path = profile.base_path.clone();
                    ui.horizontal(|ui| {
                        ui.strong("Local Path:");
                        ui.label(base_path.to_string_lossy().to_string());
                    });
                    
                    // Handle folder opening in a separate UI element
                    ui.horizontal(|ui| {
                        if ui.button("📂 Open").clicked() {
                            if let Err(e) = opener::open(&base_path) {
                                self.state.status.set_error(format!("Failed to open folder: {}", e));
                            }
                        }
                    });
                    ui.add_space(8.0);
                }
                
                // Server info display
                ui.horizontal(|ui| {
                    ui.strong("Name:");
                    ui.label(&repo.repo_name);
                });
                ui.horizontal(|ui| {
                    ui.strong("Version:");
                    ui.label(&repo.version);
                });
                
                // Mod counts
                ui.horizontal(|ui| {
                    ui.strong("Required Mods:");
                    ui.label(format!("{}", repo.required_mods.len()));
                });
                ui.horizontal(|ui| {
                    ui.strong("Optional Mods:");
                    ui.label(format!("{}", repo.optional_mods.len()));
                });

                // Server list
                if !repo.servers.is_empty() {
                    ui.add_space(8.0);
                    ui.strong("Available Servers:");
                    for server in &repo.servers {
                        ui.horizontal(|ui| {
                            ui.label("•");
                            ui.label(format!("{} ({}:{})", 
                                server.name,
                                server.address,
                                server.port
                            ));
                        });
                    }
                }

                // Launch parameters if present
                if !repo.client_parameters.is_empty() {
                    ui.add_space(8.0);
                    ui.strong("Launch Parameters:");
                    ui.label(&repo.client_parameters);
                }
            });
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui, gui_state: &GuiState, sender: Option<&Sender<CommandMessage>>) {
        // Show status
        self.state.status.show(ui);

        // Profile management section
        ui.group(|ui| {
            if self.state.profile_manager.show_editor(ui, sender) {
                if let Some(sender) = sender {
                    sender.send(CommandMessage::ConfigChanged).ok();
                }
            }
        });

        ui.add_space(8.0);

        // Connection control group
        ui.horizontal(|ui| {
            match self.state.connection_state() {
                ConnectionState::Connected => {
                    ui.label("Connected");
                    if ui.button("Disconnect").clicked() && sender.is_some() {
                        disconnect(&mut self.state, sender.unwrap());
                    }
                },
                ConnectionState::Connecting => {
                    ui.spinner();
                    ui.label("Connecting to server...");
                },
                ConnectionState::Error(error) => {
                    ui.label(format!("Connection error: {}", error));
                    if let Some(url) = self.state.profile_manager.get_current_url() {
                        if ui.button("Retry Connection").clicked() && sender.is_some() {
                            connect_to_server(&mut self.state, &url, sender.unwrap());
                        }
                    }
                },
                ConnectionState::Disconnected => {
                    ui.label("Not connected");
                    if let Some(url) = self.state.profile_manager.get_current_url() {
                        if !matches!(gui_state, GuiState::Scanning { .. } | GuiState::Syncing { .. }) {
                            if ui.button("Connect").clicked() && sender.is_some() {
                                connect_to_server(&mut self.state, &url, sender.unwrap());
                            }
                        }
                    }
                },
            }
        });

        if matches!(self.state.connection_state(), ConnectionState::Connected) {
            ui.add_space(8.0);
            self.show_server_info(ui);
            
            // Show action buttons if we have a URL
            if let Some(url) = self.state.profile_manager.get_current_url() {
                let base_path = self.state.profile_manager.get_base_path();
                match gui_state {
                    GuiState::Scanning { message } => {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Scanning mods...");
                            });
                            ui.label(message);
                        });
                    }
                    GuiState::Syncing { current_file, files_processed, total_files, progress } => {
                        ui.group(|ui| {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Syncing mods...");
                                
                                if let Some(sender) = sender {
                                    if ui.button("Cancel").clicked() {
                                        self.state.sync_cancel.store(true, Ordering::SeqCst);
                                    }
                                }
                            });
                            
                            if !current_file.is_empty() {
                                ui.label(current_file);
                            }
                            
                            ui.add(egui::ProgressBar::new(*progress)
                                .text(format!("{}/{}", files_processed, total_files)));
                        });
                    }
                    _ => show_action_buttons(ui, &mut self.state, sender, &base_path, &url),
                }
            }
        }
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
            _ => {}
        }
    }
}