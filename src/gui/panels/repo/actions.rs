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
    ui.horizontal(|ui| {
        show_scan_button(ui, state, sender, base_path);
        ui.add_space(8.0);
        show_sync_button(ui, state, sender);
    });
}

pub fn show_scan_button(
    ui: &mut egui::Ui,
    state: &mut RepoPanelState,
    sender: Option<&Sender<CommandMessage>>,
    base_path: &PathBuf,
) {
    if ui.button("Scan Mods").clicked() {
        if (!state.is_connected()) {
            state.status().set_error("No repository connected");
            return;
        }
        
        if base_path.to_str().unwrap_or("").trim().is_empty() {
            state.status().set_error("Base path is required");
            return;
        }

        // Get all required data before spawning thread
        let repo = match state.repository() {
            Some(repo) => repo.clone(),
            None => {
                state.status().set_error("Repository not available");
                return;
            }
        };

        let profile = match state.profile_manager().get_selected_profile() {
            Some(profile) => profile.clone(),
            None => {
                state.status().set_error("No profile selected");
                return;
            }
        };

        if let Some(sender) = sender {
            let repo_url = profile.repo_url.clone();
            let base_path = base_path.clone();
            let sender_clone = sender.clone();
            
            sender.send(CommandMessage::ScanStarted).ok();
            
            std::thread::spawn(move || {
                let mut agent = ureq::agent();
                match crate::commands::scan::scan_local_mods(
                    &mut agent,
                    &repo_url,
                    &base_path,
                    &repo,
                    &sender_clone
                ) {
                    Ok(updates) => {
                        let total_files: usize = updates.iter()
                            .map(|m| m.files.len().max(1))
                            .sum();
                        
                        if updates.is_empty() {
                            sender_clone.send(CommandMessage::ScanningStatus(
                                "All mods are up to date".into()
                            )).ok();
                        } else {
                            let msg = format!(
                                "Found {} mod(s) that need updating ({} files)",
                                updates.len(),
                                total_files
                            );
                            sender_clone.send(CommandMessage::ScanningStatus(msg)).ok();
                        }
                        std::thread::sleep(std::time::Duration::from_secs(2));
                        sender_clone.send(CommandMessage::SyncComplete).ok();
                    }
                    Err(e) => {
                        sender_clone.send(CommandMessage::SyncError(e)).ok();
                    }
                }
            });
        }
    }
}

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

        // Get all required data before spawning thread
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

        if let Some(sender) = sender {
            // Store cancel state before thread spawn
            state.sync_cancel().store(false, Ordering::SeqCst);
            state.set_scan_results(None);

            let sync_context = crate::commands::sync::SyncContext {
                cancel: state.sync_cancel().clone(),
                status_sender: Some(sender.clone()),
            };

            let repo_url = profile.repo_url;
            let sender = sender.clone();

            std::thread::spawn(move || {
                let mut agent = ureq::agent();
                match crate::commands::sync::sync_with_context(
                    &mut agent,
                    &repo_url,
                    &base_path,
                    false,
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
