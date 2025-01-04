use eframe::egui;
use crate::commands;
use crate::gui::state::{CommandMessage, GuiState};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;

pub struct SyncPanel {
    repo_url: String,
    base_path: String,
}

impl Default for SyncPanel {
    fn default() -> Self {
        Self {
            repo_url: String::new(),
            base_path: String::new(),
        }
    }
}

impl SyncPanel {
    fn start_sync(&self, sender: Sender<CommandMessage>) {
        let repo_url = self.repo_url.clone();
        let base_path = PathBuf::from(&self.base_path);

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
        let base_path = PathBuf::from(&self.base_path);

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
                    ui.text_edit_singleline(&mut self.repo_url);
                });
                
                ui.horizontal(|ui| {
                    ui.label("Base Path:");
                    ui.text_edit_singleline(&mut self.base_path);
                    if ui.button("Browse").clicked() {
                        // TODO: Implement file dialog
                    }
                });
                
                ui.horizontal(|ui| {
                    if ui.button("Start Sync").clicked() {
                        if let Some(sender) = sender {
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
