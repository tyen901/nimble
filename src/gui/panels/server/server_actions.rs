
use std::path::PathBuf;
use std::sync::mpsc::Sender;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use crate::gui::state::CommandMessage;

pub fn start_sync_with_context(base_path: PathBuf, repo_url: &str, sync_cancel: Arc<AtomicBool>, sender: Sender<CommandMessage>) {
    let repo_url = repo_url.to_string();
    let context = crate::commands::sync::SyncContext {
        cancel: sync_cancel,
        status_sender: Some(sender.clone()),
    };
    
    std::thread::spawn(move || {
        let mut agent = ureq::agent();
        match crate::commands::sync::sync_with_context(&mut agent, &repo_url, &base_path, false, &context) {
            Ok(()) => sender.send(CommandMessage::SyncComplete),
            Err(crate::commands::sync::Error::Cancelled) => sender.send(CommandMessage::SyncCancelled),
            Err(e) => sender.send(CommandMessage::SyncError(e.to_string())),
        }.ok();
    });
}

pub fn connect_to_server(repo_url: &str, sender: Sender<CommandMessage>) {
    let repo_url = repo_url.to_string();
    std::thread::spawn(move || {
        let mut agent = ureq::agent();
        
        // First validate the connection
        if let Err(e) = crate::repository::Repository::validate_connection(&mut agent, &repo_url) {
            sender.send(CommandMessage::ConnectionError(e)).ok();
            return;
        }

        // Then attempt to load the repository
        match crate::repository::Repository::new(&repo_url, &mut agent) {
            Ok(repo) => sender.send(CommandMessage::ConnectionComplete(repo)),
            Err(e) => sender.send(CommandMessage::ConnectionError(e.to_string())),
        }.ok();
    });
}