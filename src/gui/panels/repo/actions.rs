use eframe::egui;
use std::sync::mpsc::Sender;
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::gui::state::CommandMessage;
use crate::gui::panels::repo::state::ConnectionState;
use crate::repository::Repository;
use super::state::RepoPanelState;

pub fn show_action_buttons(
    ui: &mut egui::Ui,
    state: &mut RepoPanelState,
    sender: Option<&Sender<CommandMessage>>,
    base_path: &PathBuf,
) {
    show_sync_button(ui, state, sender);
}

// Remove show_scan_button function entirely

pub fn show_sync_button(
    ui: &mut egui::Ui,
    state: &mut RepoPanelState,
    sender: Option<&Sender<CommandMessage>>,
) {
    if ui.button("Sync Mods").clicked() {
        if (!state.is_connected()) {
            state.status().set_error("No repository connected");
            return;
        }

        let profile = match state.profile_manager().get_selected_profile() {
            Some(profile) => profile.clone(),
            None => {
                state.status().set_error("No profile selected");
                return;
            }
        };

        let base_path = profile.base_path.clone();
        if base_path.to_str().unwrap_or("").trim().is_empty() {
            state.status().set_error("Base path is required");
            return;
        }

        if let Some(sender) = sender.cloned() {
            state.sync_cancel().store(false, Ordering::SeqCst);
            let repo = state.repository().expect("Repository not available").clone();
            let repo_url = profile.repo_url.clone();
            
            sender.send(CommandMessage::SyncStarted).ok();
            
            std::thread::spawn(move || {
                let mut agent = ureq::agent();
                let sync_context = crate::commands::sync::SyncContext {
                    cancel: Arc::new(AtomicBool::new(false)),
                    status_sender: Some(sender.clone()),
                };

                match crate::commands::sync::sync_with_context(
                    &mut agent,
                    &repo_url,
                    &base_path,
                    false,
                    &sync_context
                ) {
                    Ok(()) => sender.send(CommandMessage::SyncComplete),
                    Err(crate::commands::sync::Error::Cancelled) => sender.send(CommandMessage::SyncCancelled),
                    Err(e) => sender.send(CommandMessage::SyncError(e.to_string())),
                }.ok();
            });
        }
    }
}

pub fn show_launch_button(
    ui: &mut egui::Ui,
    state: &mut RepoPanelState,
    sender: Option<&Sender<CommandMessage>>,
    base_path: &PathBuf,
) {
    let can_launch = state.has_local_data() && 
                     !base_path.to_str().unwrap_or("").trim().is_empty();

    let button = ui.add_enabled(
        can_launch,
        egui::Button::new(if state.is_offline_mode() {
            "Launch Game (Offline)"
        } else {
            "Launch Game"
        })
    );

    if button.clicked() {
        if let Some(sender) = sender {
            sender.send(CommandMessage::LaunchStarted).ok();
            let base_path = base_path.clone();
            let launch_params = state.get_launch_parameters();
            let sender_clone = sender.clone();
            
            std::thread::spawn(move || {
                if let Err(e) = crate::commands::launch::launch(
                    &base_path,
                    launch_params.as_deref()
                ) {
                    sender_clone.send(CommandMessage::LaunchError(e.to_string())).ok();
                } else {
                    sender_clone.send(CommandMessage::LaunchComplete).ok();
                }
            });
        }
    }

    if button.hovered() && !can_launch {
        button.on_hover_ui(|ui| {
            if base_path.to_str().unwrap_or("").trim().is_empty() {
                ui.label("Base path is required");
            } else if !state.has_local_data() {
                ui.label("No local repository data available. Connect once to download settings.");
            }
        });
    }
}
