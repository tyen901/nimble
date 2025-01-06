use eframe::egui;
use std::sync::mpsc::Sender;
use crate::gui::state::{CommandMessage, GuiState};
use super::state::GenSrfPanelState;
use super::actions;

pub fn render_panel(ui: &mut egui::Ui, state: &mut GenSrfPanelState, sender: Option<&Sender<CommandMessage>>, gui_state: &GuiState) {
    state.status.show(ui);

    ui.label("Select mod folder containing @mod directories:");
    state.input_path.show(ui);
    ui.add_space(4.0);
    
    ui.label("Select output folder for generated SRF files (optional):");
    state.output_path.show(ui);
    ui.add_space(8.0);

    match gui_state {
        GuiState::GeneratingSRF { progress, current_mod, mods_processed, total_mods } => {
            ui.add(egui::ProgressBar::new(*progress)
                .text(format!("{} ({}/{})", current_mod, mods_processed, total_mods)));
        },
        GuiState::Idle => {
            render_buttons(ui, state, sender);
        },
        _ => {
            ui.add_enabled(false, egui::Button::new("Generate"));
        }
    }
}

fn render_buttons(ui: &mut egui::Ui, state: &mut GenSrfPanelState, sender: Option<&Sender<CommandMessage>>) {
    if ui.button("Generate").clicked() {
        let input_path = state.input_path.path();
        let output_path = state.output_path.path();
        
        if let Err(e) = actions::validate_path(&input_path) {
            state.status.set_error(e);
        } else if let Some(sender) = sender {
            state.output_dir = if output_path.as_os_str().is_empty() {
                None
            } else {
                Some(output_path.clone())
            };

            actions::start_generation(input_path, output_path, sender.clone(), ui.ctx().clone());
        }
    }

    if let Some(output_dir) = &state.output_dir {
        if ui.button("Open Output Directory").clicked() {
            open::that(output_dir).unwrap();
        }
    }
}
