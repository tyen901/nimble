use eframe::egui;
use std::sync::mpsc::Sender;
use std::path::PathBuf;
use crate::gui::state::{CommandMessage, GuiState};
use crate::gui::widgets::{StatusDisplay, CommandHandler, PathPicker};

pub struct GenSrfPanel {
    input_path: PathPicker,
    output_path: PathPicker,
    status: StatusDisplay,
    output_dir: Option<PathBuf>,
}

impl CommandHandler for GenSrfPanel {}

impl Default for GenSrfPanel {
    fn default() -> Self {
        Self {
            input_path: PathPicker::new("Input Path:", "Select Input Directory"),
            output_path: PathPicker::new("Output Path (optional):", "Select Output Directory"),
            status: StatusDisplay::default(),
            output_dir: None,
        }
    }
}

impl GenSrfPanel {
    pub fn show(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>, state: &GuiState) {
        self.status.show(ui);

        self.input_path.show(ui);
        ui.add_space(4.0);
        self.output_path.show(ui);
        ui.add_space(8.0);

        match state {
            GuiState::GeneratingSRF { progress, current_mod, mods_processed, total_mods } => {
                ui.add(egui::ProgressBar::new(*progress)
                    .text(format!("{} ({}/{})", current_mod, mods_processed, total_mods)));
            },
            GuiState::Idle => {
                if ui.button("Generate").clicked() {
                    let input_path = self.input_path.path();
                    let output_path = self.output_path.path();
                    
                    if let Err(e) = Self::validate(&input_path) {
                        self.status.set_error(e);
                    } else if let Some(sender) = sender {
                        // Store output path before spawning thread
                        self.output_dir = if output_path.as_os_str().is_empty() {
                            None
                        } else {
                            Some(output_path.clone())
                        };

                        Self::start_generation(input_path, output_path, sender.clone(), ui.ctx().clone());
                    }
                }

                if let Some(output_dir) = &self.output_dir {
                    if ui.button("Open Output Directory").clicked() {
                        open::that(output_dir).unwrap();
                    }
                }
            },
            _ => {
                ui.add_enabled(false, egui::Button::new("Generate"));
            }
        }
    }

    fn validate(path: &PathBuf) -> Result<(), String> {
        if path.as_os_str().is_empty() {
            return Err("Input path is required".into());
        }
        Ok(())
    }

    fn start_generation(
        input_path: PathBuf,
        output_path: PathBuf,
        sender: Sender<CommandMessage>,
        ctx: egui::Context
    ) {
        let output_path = if output_path.as_os_str().is_empty() {
            None
        } else {
            Some(output_path)
        };
        
        sender.send(CommandMessage::GenSrfProgress {
            current_mod: "Starting...".to_string(),
            progress: 0.0,
            processed: 0,
            total: 0,
        }).ok();

        let progress_sender = sender.clone();

        std::thread::spawn(move || {
            let result = crate::commands::gen_srf::gen_srf(
                &input_path,
                output_path.as_deref(),
                Some(Box::new(move |current_mod, progress, processed, total| {
                    progress_sender.send(CommandMessage::GenSrfProgress {
                        current_mod,
                        progress,
                        processed,
                        total,
                    }).ok();
                    ctx.request_repaint();
                }))
            );

            match result {
                Ok(()) => sender.send(CommandMessage::GenSrfComplete),
                Err(e) => sender.send(CommandMessage::GenSrfError(e.to_string())),
            }.ok();
        });
    }
}
