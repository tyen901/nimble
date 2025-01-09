use eframe::egui;
use std::sync::mpsc::Sender;
use crate::gui::state::{CommandMessage, GuiState};
use crate::gui::panels::repo::state::ConnectionState;
use super::state::RepoPanelState;

// Remove the show_connection_status function since it's now handled in the panel

pub fn connect_to_server(state: &mut RepoPanelState, repo_url: &str, sender: &Sender<CommandMessage>) {
    state.set_connecting();
    let repo_url = repo_url.to_string();
    let sender = sender.clone();
    
    std::thread::spawn(move || {
        let mut agent = ureq::agent();
        
        if let Err(e) = crate::repository::Repository::validate_connection(&mut agent, &repo_url) {
            sender.send(CommandMessage::ConnectionError(e)).ok();
            return;
        }

        match crate::repository::Repository::new(&repo_url, &mut agent) {
            Ok(repo) => sender.send(CommandMessage::ConnectionComplete(repo)),
            Err(e) => sender.send(CommandMessage::ConnectionError(e.to_string())),
        }.ok();
    });
}

pub fn disconnect(state: &mut RepoPanelState, sender: &Sender<CommandMessage>) {
    state.disconnect();
    sender.send(CommandMessage::Disconnect).ok();
}