use eframe::egui;
use crate::gui::widgets::{PathPicker, StatusDisplay};
use crate::gui::state::{CommandMessage, GuiState};
use std::sync::mpsc::Sender;

pub struct LaunchPanel {
    path_picker: PathPicker,
    status: StatusDisplay,
}

impl Default for LaunchPanel {
    fn default() -> Self {
        Self {
            path_picker: PathPicker::new("Mods Path:", "Select Mods Directory"),
            status: StatusDisplay::default(),
        }
    }
}

impl LaunchPanel {
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

    pub fn show(&mut self, ui: &mut egui::Ui, state: &GuiState, sender: Option<&Sender<CommandMessage>>) {
        ui.heading("Launch Arma 3");
        ui.add_space(8.0);
        
        self.status.show(ui);
        
        match state {
            GuiState::Launching => {
                ui.label("Launching game...");
            },
            GuiState::Idle => {
                self.path_picker.show(ui);
                
                if ui.button("Launch Game").clicked() {
                    self.status.clear();
                    if let Err(e) = self.validate() {
                        self.status.set_error(e);
                    } else if let Some(sender) = sender {
                        sender.send(CommandMessage::LaunchStarted).ok();
                        // TODO: Implement launch logic
                    }
                }
            },
            _ => {}
        }
    }
}
