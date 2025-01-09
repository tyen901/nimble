use eframe::egui;
use crate::gui::state::{GuiState, GuiConfig, CommandMessage};
use crate::mod_cache::ModCache;
use crate::repository::Repository;
use std::sync::mpsc::Sender;
use std::sync::atomic::Ordering;
use super::state::{RepoPanelState, ConnectionState};
use super::connection::{connect_to_server, disconnect};
use super::actions::{show_action_buttons, show_scan_button, show_sync_button, show_launch_button};

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

    fn show_connection_status(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>) {
        ui.horizontal(|ui| {
            match self.state.connection_state() {
                ConnectionState::Connected => {
                    ui.label("🟢 Connected");
                    if ui.button("Disconnect").clicked() && sender.is_some() {
                        disconnect(&mut self.state, sender.unwrap());
                    }
                },
                ConnectionState::Connecting => {
                    ui.spinner();
                    ui.label("Connecting to server...");
                },
                ConnectionState::Error(_) | ConnectionState::Disconnected => {
                    if self.state.is_offline_mode() {
                        ui.label("📴 Offline Mode");
                        if let Some(url) = self.state.profile_manager.get_current_url() {
                            if ui.button("Connect").clicked() && sender.is_some() {
                                connect_to_server(&mut self.state, &url, sender.unwrap());
                            }
                        }
                    } else {
                        ui.label("❌ Not Connected");
                        if let Some(url) = self.state.profile_manager.get_current_url() {
                            if ui.button("Connect").clicked() && sender.is_some() {
                                connect_to_server(&mut self.state, &url, sender.unwrap());
                            }
                        }
                    }
                },
            }
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, gui_state: &GuiState, sender: Option<&Sender<CommandMessage>>) {
        // Show status
        self.state.status.show(ui);

        // Profile management section
        {
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

        ui.add_space(8.0);

        // Always show local repository info if available
        if self.state.has_local_data() {
            ui.add_space(8.0);
            self.show_local_info(ui);
        }

        // Connection control group
        self.show_connection_status(ui, sender);

        // Connection status and remote operations
        if matches!(self.state.connection_state(), ConnectionState::Connected) {
            ui.add_space(8.0);
            ui.separator();
            ui.add_space(8.0);
            
            // Remote repository operations
            if let Some(profile) = self.state.profile_manager.get_selected_profile().cloned() {
                let base_path = profile.base_path.clone();
                match gui_state {
                    GuiState::Scanning { .. } => self.show_scanning_ui(ui),
                    GuiState::Syncing { .. } => self.show_syncing_ui(ui),
                    _ => self.show_remote_operations(ui, sender, &base_path),
                }
            }
        }

        // Always show launch button if we have local data
        if self.state.has_local_data() {
            ui.add_space(8.0);
            let base_path = self.state.profile_manager.get_base_path();
            ui.horizontal(|ui| {
                show_launch_button(ui, &mut self.state, sender, &base_path);
            });
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

    fn show_local_info(&mut self, ui: &mut egui::Ui) {
        // Get all the data we need up front to avoid multiple borrows
        let repo_info = self.state.get_repository_for_launch().cloned();
        let profile_info = self.state.profile_manager.get_selected_profile().cloned();
        let sync_age = self.state.sync_age();

        if let Some(repo) = repo_info {
            ui.group(|ui| {
                ui.heading("Local Repository");
                ui.add_space(4.0);
                
                // Show local path info
                if let Some(profile) = profile_info {
                    let base_path = &profile.base_path;
                    ui.horizontal(|ui| {
                        ui.strong("Installation Path:");
                        ui.label(base_path.to_string_lossy().to_string());
                        if ui.button("📂 Open").clicked() {
                            if let Err(e) = opener::open(base_path) {
                                self.state.status.set_error(format!("Failed to open folder: {}", e));
                            }
                        }
                    });
                }

                // Show repository info
                ui.horizontal(|ui| {
                    ui.strong("Name:");
                    ui.label(&repo.repo_name);
                });
                ui.horizontal(|ui| {
                    ui.strong("Version:");
                    ui.label(&repo.version);
                });

                // Show mod counts
                ui.horizontal(|ui| {
                    ui.strong("Required Mods:");
                    ui.label(format!("{}", repo.required_mods.len()));
                });
                ui.horizontal(|ui| {
                    ui.strong("Optional Mods:");
                    ui.label(format!("{}", repo.optional_mods.len()));
                });

                // Show last sync time if available
                if let Some(duration) = sync_age {
                    ui.horizontal(|ui| {
                        ui.strong("Last Synced:");
                        ui.label(format!("{} hours ago", duration.num_hours()));
                    });
                }
            });
        }
    }

    fn show_scanning_ui(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Scanning mods...");
            });
            if let Some(results) = &self.state.scan_results {
                ui.label(format!("Found {} mod(s) that need updating", results.len()));
            }
        });
    }

    fn show_syncing_ui(&mut self, ui: &mut egui::Ui) {
        ui.group(|ui| {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Syncing mods...");
                
                if ui.button("Cancel").clicked() {
                    self.state.sync_cancel.store(true, Ordering::SeqCst);
                }
            });
        });
    }

    fn show_remote_operations(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>, base_path: &std::path::Path) {
        ui.group(|ui| {
            ui.heading("Remote Operations");
            ui.horizontal(|ui| {
                show_scan_button(ui, &mut self.state, sender, &base_path.to_path_buf());
                ui.add_space(8.0);
                show_sync_button(ui, &mut self.state, sender);
            });
        });
    }
}