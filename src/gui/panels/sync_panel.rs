use eframe::egui;
use crate::commands;
use crate::gui::state::{CommandMessage, GuiState, GuiConfig};
use crate::gui::widgets::PathPicker;
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;

pub struct SyncPanel {
    repo_url: String,
    path_picker: PathPicker,
    error: Option<String>,
}

impl Default for SyncPanel {
    fn default() -> Self {
        Self {
            repo_url: String::new(),
            path_picker: PathPicker::new("Base Path:", "Select Mods Directory"),
            error: None,
        }
    }
}

impl SyncPanel {
    pub fn from_config(config: &GuiConfig) -> Self {
        let mut path_picker = PathPicker::new("Base Path:", "Select Mods Directory");
        if config.base_path.exists() {
            path_picker.path = config.base_path.display().to_string();
        }
        
        Self {
            repo_url: config.repo_url.clone(),
            path_picker,
            error: None,
        }
    }

    pub fn repo_url(&self) -> &str {
        &self.repo_url
    }

    pub fn base_path(&self) -> PathBuf {
        self.path_picker.path()
    }

    pub fn update_config(&self, config: &mut GuiConfig) {
        config.repo_url = self.repo_url.clone();
        config.base_path = self.path_picker.path();
    }

    fn validate(&self) -> Result<(), String> {
        if self.repo_url.trim().is_empty() {
            return Err("Repository URL is required".into());
        }
        if self.path_picker.path().to_str().unwrap_or("").trim().is_empty() {
            return Err("Base path is required".into());
        }
        Ok(())
    }

    fn start_sync(&self, sender: Sender<CommandMessage>) {
        let repo_url = self.repo_url.clone();
        let base_path = PathBuf::from(self.path_picker.path());

        thread::spawn(move || {
            let mut agent = ureq::agent();
            
            match commands::sync::sync(&mut agent, &repo_url, &base_path, false) {
                Ok(()) => sender.send(CommandMessage::SyncComplete),
                Err(e) => sender.send(CommandMessage::SyncError(e.to_string())),
            }.ok(); // Ignore send errors - UI might be closed
        });
    }

    pub fn start_sync_dry_run(&self, sender: Sender<CommandMessage>) {
        let repo_url = self.repo_url.clone();
        let base_path = PathBuf::from(self.path_picker.path());

        thread::spawn(move || {
            let mut agent = ureq::agent();
            
            match commands::sync::sync(&mut agent, &repo_url, &base_path, true) {
                Ok(()) => sender.send(CommandMessage::SyncComplete),
                Err(e) => sender.send(CommandMessage::SyncError(e.to_string())),
            }.ok();
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, state: &GuiState, sender: Option<&Sender<CommandMessage>>) {
        ui.heading("Sync Mods");
        ui.add_space(8.0);
        
        if let Some(error) = &self.error {
            ui.colored_label(ui.style().visuals.error_fg_color, error);
            ui.add_space(8.0);
        }

        match state {
            GuiState::Syncing { progress, current_file, files_processed, total_files } => {
                ui.label(format!("Syncing: {} / {} files", files_processed, total_files));
                ui.label(format!("Current file: {}", current_file));
                ui.add(egui::ProgressBar::new(*progress)
                    .show_percentage()
                    .animate(true));
            },
            GuiState::Idle => {
                ui.horizontal(|ui| {
                    ui.label("Repository URL:");
                    if ui.text_edit_singleline(&mut self.repo_url).changed() {
                        if let Some(sender) = sender {
                            sender.send(CommandMessage::ConfigChanged).ok();
                        }
                    }
                });
                
                if self.path_picker.show(ui).is_some() {
                    if let Some(sender) = sender {
                        sender.send(CommandMessage::ConfigChanged).ok();
                    }
                }
                
                ui.horizontal(|ui| {
                    if ui.button("Start Sync").clicked() {
                        self.error = None;
                        if let Err(e) = self.validate() {
                            self.error = Some(e);
                        } else if let Some(sender) = sender {
                            self.start_sync(sender.clone());
                        }
                    }
                    if ui.button("Dry Run").clicked() {
                        if let Some(sender) = sender {
                            self.start_sync_dry_run(sender.clone());
                        }
                    }
                });
            },
            _ => {}
        }
    }
}
