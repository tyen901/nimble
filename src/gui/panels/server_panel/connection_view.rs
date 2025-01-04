use eframe::egui;
use std::sync::mpsc::Sender;
use crate::gui::state::CommandMessage;

pub struct ConnectionView {
    pub repo_url: String,
    error: Option<String>,
}

impl Default for ConnectionView {
    fn default() -> Self {
        Self {
            repo_url: String::new(),
            error: None,
        }
    }
}

impl ConnectionView {
    pub fn show(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>) {
        if let Some(error) = &self.error {
            ui.colored_label(ui.style().visuals.error_fg_color, error);
            ui.add_space(8.0);
        }

        ui.horizontal(|ui| {
            ui.label("Repository URL:");
            ui.text_edit_singleline(&mut self.repo_url);
        });

        if ui.button("Connect").clicked() {
            self.error = None;
            if let Err(e) = self.validate() {
                self.error = Some(e);
            } else if let Some(sender) = sender {
                sender.send(CommandMessage::ConnectionStarted).ok();
                self.connect_to_server(sender.clone());
            }
        }
    }

    fn validate(&self) -> Result<(), String> {
        if self.repo_url.trim().is_empty() {
            return Err("Repository URL is required".into());
        }
        Ok(())
    }

    fn connect_to_server(&self, sender: Sender<CommandMessage>) {
        let repo_url = self.repo_url.clone();
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
