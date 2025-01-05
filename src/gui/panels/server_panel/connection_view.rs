use eframe::egui;
use std::sync::mpsc::Sender;
use crate::gui::state::CommandMessage;
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
    pub fn show(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>) {
        self.status.show(ui);

        ui.horizontal(|ui| {
            ui.label("Repository URL:");
            ui.text_edit_singleline(&mut self.repo_url);
        });

        if ui.button("Connect").clicked() {
            // Clone values before using in closures
            let repo_url = self.repo_url.clone();
            let status = &mut self.status;
            
            <Self as CommandHandler>::handle_validation(
                || Self::validate(&repo_url),
                |e| status.set_error(e),
                |s| Self::connect_to_server(&repo_url, s.unwrap().clone()),
                sender
            );
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
