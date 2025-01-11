use eframe::egui;
use std::sync::mpsc::Sender;
use crate::gui::state::{CommandMessage, GuiState};
use crate::gui::panels::repo::state::ConnectionState;
use crate::repository::Repository;
use super::state::RepoPanelState;
use crate::mod_cache::ModCache;

fn ensure_url_has_scheme(url: &str) -> String {
    if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{}", url)
    } else {
        url.to_string()
    }
}

pub fn connect_to_server(state: &mut RepoPanelState, repo_url: &str, sender: &Sender<CommandMessage>) {
    let profile = match state.profile_manager().get_selected_profile().cloned() {
        Some(p) => p,
        None => {
            eprintln!("Connection failed: No profile selected");
            sender.send(CommandMessage::ConnectionError("No profile selected".to_string())).ok();
            return;
        }
    };

    // Load existing cache but don't update it
    if let Ok(cache) = ModCache::from_disk_or_empty(&profile.base_path) {
        state.load_cache(&cache);
    }

    // Ensure URL has a scheme
    let repo_url = ensure_url_has_scheme(repo_url);
    println!("Attempting to connect to repository: {}", repo_url);
    
    // Start connection process
    state.set_connecting();
    let sender = sender.clone();
    
    std::thread::spawn(move || {
        let mut agent = ureq::agent();
        match Repository::new(&repo_url, &mut agent) {
            Ok(repo) => {
                println!("Successfully connected to repository {}", repo_url);
                sender.send(CommandMessage::ConnectionComplete(repo))
            },
            Err(e) => {
                eprintln!("Failed to connect to repository {}: {}", repo_url, e);
                sender.send(CommandMessage::ConnectionError(format!(
                    "Failed to connect to {}: {}", 
                    repo_url, 
                    e.to_string()
                )))
            },
        }.ok();
    });
}

pub fn disconnect(state: &mut RepoPanelState, sender: &Sender<CommandMessage>) {
    state.disconnect();
    sender.send(CommandMessage::Disconnect).ok();
}