use eframe::egui;
use crate::gui::state::{GuiState, CommandMessage};
use super::super::state::{RepoPanelState, OperationState};  // Add OperationState
use super::super::actions;
use std::sync::mpsc::Sender;
use std::path::PathBuf;

pub struct OperationsView;

impl OperationsView {
    pub fn show(
        ui: &mut egui::Ui,
        state: &mut RepoPanelState,
        gui_state: &GuiState,
        sender: Option<&Sender<CommandMessage>>
    ) {
        let base_path = state.profile_manager().get_base_path();

        if let Some(repo) = state.repository() {
            // Show repository info without base path
            super::repository_info::RepositoryInfoView::show(ui, repo, None);

            ui.add_space(8.0);

            ui.group(|ui| {
                ui.heading("Operations");
                ui.add_enabled_ui(!state.is_busy(), |ui| {
                    actions::show_action_buttons(ui, state, sender, &base_path);
                });

                // Show operation status if busy
                if state.is_busy() {
                    ui.add_space(4.0);
                    match state.operation_state {
                        OperationState::Scanning => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Scanning for updates...");
                            });
                        },
                        OperationState::Syncing => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Syncing repository...");
                            });
                        },
                        OperationState::Launching => {
                            ui.horizontal(|ui| {
                                ui.spinner();
                                ui.label("Launching game...");
                            });
                        },
                        _ => {}
                    }
                }
            });

            // Show any scanning results
            if let Some(results) = &state.scan_results {
                ui.add_space(8.0);
                ui.group(|ui| {
                    ui.heading("Scan Results");
                    for update in results {
                        ui.horizontal(|ui| {
                            ui.label("•");
                            ui.label(&update.name);
                            ui.label(format!(
                                "({})",
                                if update.files.len() > 1 {
                                    format!("{} files", update.files.len())
                                } else {
                                    "1 file".to_string()
                                }
                            ));
                        });
                    }
                });
            }
        }
    }
}
