use std::path::PathBuf;
use std::sync::mpsc::Sender;
use eframe::egui;
use crate::gui::state::CommandMessage;

pub fn validate_path(path: &PathBuf) -> Result<(), String> {
    if path.as_os_str().is_empty() {
        return Err("Input path is required".into());
    }
    if !path.exists() {
        return Err("Input path does not exist".into());
    }
    Ok(())
}

pub fn start_generation(
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
