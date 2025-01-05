use eframe::egui;
use std::sync::mpsc::Sender;
use std::path::PathBuf;
use crate::gui::state::{CommandMessage, GuiState};
use crate::gui::widgets::{StatusDisplay, CommandHandler, PathPicker};

pub struct GenSrfPanel {
    input_path: PathPicker,
    output_path: PathPicker,
    status: StatusDisplay,
}

impl CommandHandler for GenSrfPanel {}

impl Default for GenSrfPanel {
    fn default() -> Self {
        Self {
            input_path: PathPicker::new("Input Path:", "Select Input Directory"),
            output_path: PathPicker::new("Output Path (optional):", "Select Output Directory"),
            status: StatusDisplay::default(),
        }
    }
}

impl GenSrfPanel {
    pub fn show(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>, _state: &GuiState) {
        self.status.show(ui);

        self.input_path.show(ui);
        ui.add_space(4.0);
        self.output_path.show(ui);
        ui.add_space(8.0);

        if ui.button("Generate").clicked() {
            // Clone paths before the validation to avoid borrow issues
            let input_path = self.input_path.path();
            let output_path = self.output_path.path();
            let status = &mut self.status;
            
            <Self as CommandHandler>::handle_validation(
                || Self::validate(&input_path),
                |e| status.set_error(e),
                |s| Self::start_generation(input_path.clone(), output_path.clone(), s.unwrap().clone()),
                sender
            );
        }
    }

    fn validate(path: &PathBuf) -> Result<(), String> {
        if path.as_os_str().is_empty() {
            return Err("Input path is required".into());
        }
        Ok(())
    }

    fn start_generation(input_path: PathBuf, output_path: PathBuf, sender: Sender<CommandMessage>) {
        let output_path = if output_path.as_os_str().is_empty() {
            None
        } else {
            Some(output_path)
        };
        
        std::thread::spawn(move || {
            match crate::commands::gen_srf::gen_srf(&input_path, output_path.as_deref()) {
                Ok(()) => sender.send(CommandMessage::GenSrfComplete),
                Err(e) => sender.send(CommandMessage::GenSrfError(e.to_string())),
            }.ok();
        });
    }
}
