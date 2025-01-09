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
    repo_url: &str,
) {
    ui.horizontal(|ui| {
        show_scan_button(ui, state, sender, base_path, repo_url);
        ui.add_space(8.0);
        show_sync_button(ui, state, sender, base_path, repo_url);
        ui.add_space(8.0);
        show_launch_button(ui, state, sender, base_path);
    });
}

fn show_scan_button(
    ui: &mut egui::Ui,
    state: &mut RepoPanelState,
    sender: Option<&Sender<CommandMessage>>,
    base_path: &PathBuf,
    repo_url: &str,
) {
    if ui.button("Scan Mods").clicked() {
        if !state.is_connected() {
            state.status().set_error("No repository connected");
            return;
        }
        
        if base_path.to_str().unwrap_or("").trim().is_empty() {
            state.status().set_error("Base path is required");
            return;
        }

        if let Some(sender) = sender {
            let repo = state.repository().unwrap().clone();
            let repo_url = repo_url.to_string();
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

fn show_sync_button(
    ui: &mut egui::Ui,
    state: &mut RepoPanelState,
    sender: Option<&Sender<CommandMessage>>,
    base_path: &PathBuf,
    repo_url: &str,
) {
    if ui.button("Sync Mods").clicked() {
        if !state.is_connected() {
            state.status().set_error("No repository connected");
            return;
        }
        
        if base_path.to_str().unwrap_or("").trim().is_empty() {
            state.status().set_error("Base path is required");
            return;
        }
        
        if let Some(sender) = sender {
            state.sync_cancel().store(false, Ordering::SeqCst);
            state.set_scan_results(None);
            
            let sync_context = crate::commands::sync::SyncContext {
                cancel: state.sync_cancel().clone(),
                status_sender: Some(sender.clone()),
            };

            let repo_url = repo_url.to_string();
            let base_path = base_path.clone();
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

fn show_launch_button(
    ui: &mut egui::Ui,
    state: &mut RepoPanelState,
    sender: Option<&Sender<CommandMessage>>,
    base_path: &PathBuf,
) {
    if ui.button("Launch Game").clicked() {
        if base_path.to_str().unwrap_or("").trim().is_empty() {
            state.status().set_error("Base path is required");
            return;
        }
        
        if let Some(sender) = sender {
            sender.send(CommandMessage::LaunchStarted).ok();
            let base_path = base_path.clone();
            let sender_clone = sender.clone();
            
            std::thread::spawn(move || {
                if let Err(e) = crate::commands::launch::launch(&base_path) {
                    sender_clone.send(CommandMessage::LaunchError(e.to_string())).ok();
                } else {
                    sender_clone.send(CommandMessage::LaunchComplete).ok();
                }
            });
        }
    }
}
