use eframe::egui;
use crate::commands;
use crate::gui::state::{CommandMessage, GuiState};
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::thread;

pub struct GenSrfPanel {
    base_path: String,
}

impl Default for GenSrfPanel {
    fn default() -> Self {
        Self {
            base_path: String::new(),
        }
    }
}

impl GenSrfPanel {
    fn start_gen_srf(&self, sender: Sender<CommandMessage>) {
        let base_path = PathBuf::from(&self.base_path);

        thread::spawn(move || {
            match commands::gen_srf::gen_srf(&base_path) {
                Ok(()) => sender.send(CommandMessage::GenSrfComplete),
                Err(e) => sender.send(CommandMessage::GenSrfError(e.to_string())),
            }.ok();
        });
    }

    pub fn show(&mut self, ui: &mut egui::Ui, state: &GuiState, sender: Option<&Sender<CommandMessage>>) {
        ui.heading("Generate SRF");
        ui.add_space(8.0);
        
        match state {
            GuiState::GeneratingSRF { progress, current_mod, mods_processed, total_mods } => {
                ui.label(format!("Processing: {} / {} mods", mods_processed, total_mods));
                ui.label(format!("Current mod: {}", current_mod));
                ui.add(egui::ProgressBar::new(*progress)
                    .show_percentage()
                    .animate(true));
            },
            GuiState::Idle => {
                ui.horizontal(|ui| {
                    ui.label("Mods Path:");
                    ui.text_edit_singleline(&mut self.base_path);
                    if ui.button("Browse").clicked() {
                        // TODO: Implement file dialog
                    }
                });
                
                if ui.button("Generate SRF").clicked() {
                    if let Some(sender) = sender {
                        self.start_gen_srf(sender.clone());
                    }
                }
            },
            _ => {}
        }
    }
}
