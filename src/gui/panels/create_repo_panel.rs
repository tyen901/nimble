use eframe::egui;
use std::path::{PathBuf, Path};
use std::fs;
use crate::repository::{Repository, Mod, Server};
use crate::md5_digest::Md5Digest;
use crate::gui::widgets::PathPicker;
use crate::gui::widgets::StatusDisplay;
use crate::gui::state::GuiConfig;  // Add this import
use walkdir::WalkDir;
use semver::Version;

pub struct CreateRepoPanel {
    repo: Repository,
    base_path: PathPicker,
    status: StatusDisplay,
    last_scanned_path: Option<PathBuf>,
    generate_srf: bool,
    auto_increment_version: bool,
    show_update_prompt: bool,
    pending_mods: Option<Vec<Mod>>,
    config: Option<GuiConfig>,
}

impl Default for CreateRepoPanel {
    fn default() -> Self {
        Self {
            repo: Repository {
                repo_name: String::new(),
                checksum: String::new(),
                required_mods: Vec::new(),
                optional_mods: Vec::new(),
                client_parameters: "-noPause -noSplash -skipIntro".to_string(),
                repo_basic_authentication: None,
                version: "1.0.0".to_string(),
                servers: Vec::new(),
            },
            base_path: PathPicker::new("Repository Path:", "Select Repository Directory"),
            status: StatusDisplay::default(),
            last_scanned_path: None,
            generate_srf: false,
            auto_increment_version: true,
            show_update_prompt: false,
            pending_mods: None,
            config: None,
        }
    }
}

impl CreateRepoPanel {
    pub fn from_config(config: &GuiConfig) -> Self {
        let mut panel = Self {
            repo: Repository {
                repo_name: String::new(),
                checksum: String::new(),
                required_mods: Vec::new(),
                optional_mods: Vec::new(),
                client_parameters: "-noPause -noSplash -skipIntro".to_string(),
                repo_basic_authentication: None,
                version: "1.0.0".to_string(),
                servers: Vec::new(),
            },
            base_path: PathPicker::new("Repository Path:", "Select Repository Directory"),
            status: StatusDisplay::default(),
            last_scanned_path: None,
            generate_srf: false,
            auto_increment_version: true,
            show_update_prompt: false,
            pending_mods: None,
            config: Some(config.clone()),
        };

        // Initialize with last path if available
        if let Some(path) = config.last_repo_path() {
            if path.exists() {
                panel.base_path.set_path(path);
                panel.scan_mods(path);
            }
        }

        panel
    }

    fn update_config(&mut self) {
        if let Some(config) = &mut self.config {
            config.set_last_repo_path(Some(self.base_path.path().to_path_buf()));
        }
    }

    fn scan_mods(&mut self, path: &PathBuf) {
        if self.last_scanned_path.as_ref() == Some(path) {
            return;
        }

        // Try to load existing repo.json
        let repo_file = path.join("repo.json");
        if repo_file.exists() {
            match fs::read_to_string(&repo_file) {
                Ok(contents) => {
                    match serde_json::from_str::<Repository>(&contents) {
                        Ok(loaded_repo) => {
                            self.repo = loaded_repo;
                            self.status.set_info("Loaded existing repo.json");
                            self.scan_for_changes(path);
                        },
                        Err(e) => {
                            self.status.set_error(format!("Failed to parse repo.json: {}", e));
                            return;
                        }
                    }
                },
                Err(e) => {
                    self.status.set_error(format!("Failed to read repo.json: {}", e));
                    return;
                }
            }
        } else {
            self.scan_and_update_mods(path);
            self.status.set_info(format!("Found {} mods", self.repo.required_mods.len()));
        }

        self.last_scanned_path = Some(path.clone());
    }

    fn scan_for_changes(&mut self, path: &PathBuf) {
        // Scan directory for @mod folders
        let mod_dirs: Vec<_> = WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_dir() && e.file_name().to_string_lossy().starts_with('@'))
            .collect();

        // Create new mods list
        let mut new_mods = Vec::new();
        for entry in mod_dirs {
            let mod_name = entry.file_name().to_string_lossy().to_string();
            
            let mod_entry = Mod {
                mod_name,
                checksum: Md5Digest::default(),
                enabled: true,
            };

            new_mods.push(mod_entry);
        }

        // Check if mods list is different
        if new_mods.len() != self.repo.required_mods.len() || 
           new_mods.iter().any(|m| !self.repo.required_mods.iter().any(|rm| rm.mod_name == m.mod_name)) {
            self.pending_mods = Some(new_mods);
            self.show_update_prompt = true;
            self.status.set_info("Found changes in mods. Choose whether to update.");
        }
    }

    fn scan_and_update_mods(&mut self, path: &PathBuf) {
        // Scan directory for @mod folders
        let mod_dirs: Vec<_> = WalkDir::new(path)
            .min_depth(1)
            .max_depth(1)
            .into_iter()
            .filter_map(Result::ok)
            .filter(|e| e.file_type().is_dir() && e.file_name().to_string_lossy().starts_with('@'))
            .collect();

        // Update required_mods list while preserving existing mod settings
        let mut new_mods = Vec::new();
        for entry in mod_dirs {
            let mod_name = entry.file_name().to_string_lossy().to_string();
            
            // Try to find existing mod config
            let existing_mod = self.repo.required_mods.iter()
                .find(|m| m.mod_name == mod_name)
                .cloned();

            // Use existing config or create new one
            let mod_entry = existing_mod.unwrap_or_else(|| Mod {
                mod_name,
                checksum: Md5Digest::default(),
                enabled: true,
            });

            new_mods.push(mod_entry);
        }

        // Replace mods list with updated one
        self.repo.required_mods = new_mods;
        
        if self.auto_increment_version {
            if let Ok(mut version) = Version::parse(&self.repo.version) {
                version.patch += 1;
                self.repo.version = version.to_string();
            }
        }
    }

    fn generate_srf_files(&self, path: &Path) -> Result<(), String> {
        crate::commands::gen_srf::gen_srf(
            path,
            Some(path),
            Some(Box::new(|current_mod, _, _, _| {
                println!("Generating SRF for {}", current_mod);
            }))
        ).map_err(|e| e.to_string())
    }

    // Add this method to access base_path
    pub fn get_current_path(&self) -> PathBuf {
        self.base_path.path().to_path_buf()
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        self.status.show(ui);
        ui.add_space(8.0);

        // Repository Path Selection
        ui.heading("Repository Setup");
        
        let prev_path = self.base_path.path().to_path_buf();
        self.base_path.show(ui);
        
        // Check if path changed and is valid
        let current_path = self.base_path.path();
        if current_path != prev_path && !current_path.as_os_str().is_empty() && current_path.exists() {
            self.scan_mods(&current_path);
            self.update_config();
        }

        ui.add_space(8.0);

        // Main Repository Settings
        ui.horizontal(|ui| {
            ui.vertical(|ui| {
                // Basic Settings
                ui.group(|ui| {
                    ui.heading("Basic Settings");
                    ui.horizontal(|ui| {
                        ui.label("Repository Name:");
                        ui.text_edit_singleline(&mut self.repo.repo_name);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Client Parameters:");
                        ui.text_edit_singleline(&mut self.repo.client_parameters);
                    });

                    ui.horizontal(|ui| {
                        ui.label("Version:");
                        ui.text_edit_singleline(&mut self.repo.version);
                    });
                });

                // Options
                ui.group(|ui| {
                    ui.heading("Options");
                    ui.checkbox(&mut self.generate_srf, "Generate SRF files");
                    ui.checkbox(&mut self.auto_increment_version, "Auto-increment version");
                });
            });

            ui.vertical(|ui| {
                // Mods Section with Update UI
                ui.group(|ui| {
                    ui.heading("Required Mods");
                    
                    // Always show the mods list, whether there are changes or not
                    egui::ScrollArea::vertical()
                        .max_height(200.0)
                        .show(ui, |ui| {
                            if !self.repo.required_mods.is_empty() {
                                for mod_entry in &self.repo.required_mods {
                                    ui.horizontal(|ui| {
                                        ui.label(&mod_entry.mod_name);
                                        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                                            ui.label(if mod_entry.enabled { "✓" } else { "✗" });
                                        });
                                    });
                                }
                            } else {
                                ui.label("No mods found");
                            }
                        });

                    // Show update prompt above the list if changes are detected
                    if self.show_update_prompt {
                        ui.separator();
                        ui.horizontal(|ui| {
                            ui.label("Changes detected!");
                            if ui.button("Update List").clicked() {
                                if let Some(new_mods) = self.pending_mods.take() {
                                    self.repo.required_mods = new_mods;
                                    if self.auto_increment_version {
                                        if let Ok(mut version) = Version::parse(&self.repo.version) {
                                            version.patch += 1;
                                            self.repo.version = version.to_string();
                                        }
                                    }
                                    self.status.set_info("Updated mod list");
                                }
                                self.show_update_prompt = false;
                            }
                            if ui.button("Keep Existing").clicked() {
                                self.pending_mods = None;
                                self.show_update_prompt = false;
                                self.status.set_info("Kept existing mod list");
                            }
                        });
                    }
                });
            });
        });

        // Server Configuration
        ui.group(|ui| {
            ui.heading("Servers");
            for server in &mut self.repo.servers {
                ui.horizontal(|ui| {
                    ui.label("Name:");
                    ui.text_edit_singleline(&mut server.name);
                    ui.label("Address:");
                    if ui.button(server.address.to_string()).clicked() {
                        if let Ok(new_addr) = "127.0.0.1".parse() {
                            server.address = new_addr;
                        }
                    }
                    ui.label("Port:");
                    ui.add(egui::DragValue::new(&mut server.port).speed(1));
                    ui.checkbox(&mut server.battle_eye, "BattlEye");
                });
            }
            if ui.button("Add Server").clicked() {
                self.repo.servers.push(Server {
                    name: String::new(),
                    address: "127.0.0.1".parse().unwrap(),
                    port: 2302,
                    password: String::new(),
                    battle_eye: true,
                });
            }
        });

        ui.separator();

        // Save Button
        if ui.button("Save repo.json").clicked() {
            let path = self.base_path.path();
            if path.exists() {
                // First generate SRF files if enabled
                if self.generate_srf {
                    match self.generate_srf_files(&path) {
                        Ok(_) => self.status.set_info("Generated SRF files successfully"),
                        Err(e) => {
                            self.status.set_error(format!("Failed to generate SRF files: {}", e));
                            return;
                        }
                    }
                }

                // Save repo.json directly in the mods folder
                match std::fs::File::create(path.join("repo.json")) {
                    Ok(file) => {
                        if serde_json::to_writer_pretty(file, &self.repo).is_ok() {
                            self.status.set_info("Saved repo.json successfully");
                        } else {
                            self.status.set_error("Failed to write repo.json");
                        }
                    },
                    Err(e) => self.status.set_error(format!("Failed to create repo.json: {}", e)),
                }
            }
        }
    }
}
