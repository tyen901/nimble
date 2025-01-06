use eframe::egui;
use std::sync::mpsc::Sender;
use crate::gui::state::{CommandMessage, GuiState};
use crate::gui::widgets::{StatusDisplay, CommandHandler};

pub struct ConnectionView {
    pub repo_url: String,
    status: StatusDisplay,
}

impl CommandHandler for ConnectionView {}

impl Default for ConnectionView {
    fn default() -> Self {
        Self {
            repo_url: String::new(),
            status: StatusDisplay::default(),
        }
    }
}

impl ConnectionView {
    pub fn show(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>, state: &GuiState) {
        self.status.show(ui);

        if matches!(state, GuiState::Connecting) {
            ui.horizontal(|ui| {
                ui.spinner();
                ui.label("Connecting to server...");
            });
            return;
        }

        ui.horizontal(|ui| {
            ui.label("Repository URL:");
            ui.text_edit_singleline(&mut self.repo_url);
        });

        if ui.button("Connect").clicked() && sender.is_some() {
            let repo_url = self.repo_url.clone();
            let sender = sender.unwrap().clone();
            
            // Signal connection started before spawning thread
            sender.send(CommandMessage::ConnectionStarted).ok();
            
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
    }

    fn validate(repo_url: &str) -> Result<(), String> {
        if repo_url.trim().is_empty() {
            return Err("Repository URL is required".into());
        }
        Ok(())
    }

    fn connect_to_server(repo_url: &str, sender: Sender<CommandMessage>) {
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
}
