use eframe::egui;
use std::sync::mpsc::Sender;
use crate::gui::state::{CommandMessage, GuiState};
use crate::gui::panels::repo::state::ConnectionState;
use crate::repository::Repository;
use super::state::RepoPanelState;
use crate::mod_cache::ModCache;

pub fn connect_to_server(state: &mut RepoPanelState, repo_url: &str, sender: &Sender<CommandMessage>) {
    let profile = match state.profile_manager().get_selected_profile().cloned() {
        Some(p) => p,
        None => {
            sender.send(CommandMessage::ConnectionError("No profile selected".to_string())).ok();
            return;
        }
    };

    // Load existing cache but don't update it
    if let Ok(cache) = ModCache::from_disk_or_empty(&profile.base_path) {
        state.load_cache(&cache);
    }

    // Start connection process
    state.set_connecting();
    let repo_url = repo_url.to_string();
    let sender = sender.clone();
    
    std::thread::spawn(move || {
        let mut agent = ureq::agent();
        match Repository::new(&repo_url, &mut agent) {
            Ok(repo) => sender.send(CommandMessage::ConnectionComplete(repo)),
            Err(e) => sender.send(CommandMessage::ConnectionError(e.to_string())),
        }.ok();
    });
}

pub fn disconnect(state: &mut RepoPanelState, sender: &Sender<CommandMessage>) {
    state.disconnect();
    sender.send(CommandMessage::Disconnect).ok();
}