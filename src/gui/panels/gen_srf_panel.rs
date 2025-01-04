use eframe::egui;
use crate::commands;
use crate::gui::state::{CommandMessage, GuiState};
use crate::gui::widgets::{PathPicker, StatusDisplay};
use std::sync::mpsc::Sender;
use std::thread;

pub struct GenSrfPanel {
    path_picker: PathPicker,
    status: StatusDisplay,
}

impl Default for GenSrfPanel {
    fn default() -> Self {
        Self {
            path_picker: PathPicker::new("Mods Path:", "Select Mods Directory"),
            status: StatusDisplay::default(),
        }
    }
}

impl GenSrfPanel {
    fn validate(&self) -> Result<(), String> {
        let path = self.path_picker.path();
        if !path.exists() {
            return Err("Mods path does not exist".into());
        }
        if !path.is_dir() {
            return Err("Mods path must be a directory".into());
        }
        Ok(())
    }

    fn start_gen_srf(&self, sender: Sender<CommandMessage>) {
        let base_path = self.path_picker.path();

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
        
        self.status.show(ui);
        
        match state {
            GuiState::GeneratingSRF { progress, current_mod, mods_processed, total_mods } => {
                ui.label(format!("Processing: {} / {} mods", mods_processed, total_mods));
                ui.label(format!("Current mod: {}", current_mod));
                ui.add(egui::ProgressBar::new(*progress)
                    .show_percentage()
                    .animate(true));
            },
            GuiState::Idle => {
                self.path_picker.show(ui);
                
                if ui.button("Generate SRF").clicked() {
                    self.status.clear();
                    if let Err(e) = self.validate() {
                        self.status.set_message(e, true);
                    } else if let Some(sender) = sender {
                        self.start_gen_srf(sender.clone());
                    }
                }
            },
            _ => {}
        }
    }
}
