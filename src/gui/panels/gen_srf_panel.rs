use eframe::egui;
use std::sync::mpsc::Sender;
use std::path::PathBuf;
use crate::gui::state::{CommandMessage, GuiState};
use crate::gui::widgets::{StatusDisplay, CommandHandler};

pub struct GenSrfPanel {
    path: String,
    status: StatusDisplay,
}

impl CommandHandler for GenSrfPanel {}

impl Default for GenSrfPanel {
    fn default() -> Self {
        Self {
            path: String::new(),
            status: StatusDisplay::default(),
        }
    }
}

impl GenSrfPanel {
    pub fn show(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>, _state: &GuiState) {
        self.status.show(ui);

        ui.horizontal(|ui| {
            ui.label("Path:");
            ui.text_edit_singleline(&mut self.path);
        });
        
        ui.add_space(8.0);

        if ui.button("Generate").clicked() {
            // Clone path here before the closures
            let path = self.path.clone();
            let status = &mut self.status;
            
            <Self as CommandHandler>::handle_validation(
                || Self::validate(&path),
                |e| status.set_error(e),
                |s| Self::start_generation(&path, s.unwrap().clone()),
                sender
            );
        }
    }

    fn validate(path: &str) -> Result<(), String> {
        if path.trim().is_empty() {
            return Err("Path is required".into());
        }
        Ok(())
    }

    fn start_generation(path: &str, sender: Sender<CommandMessage>) {
        let path = PathBuf::from(path);
        std::thread::spawn(move || {
            match crate::commands::gen_srf::gen_srf(&path) {
                Ok(()) => sender.send(CommandMessage::GenSrfComplete),
                Err(e) => sender.send(CommandMessage::GenSrfError(e.to_string())),
            }.ok();
        });
    }
}
