use eframe::egui;
use std::time::{Duration, Instant};

pub struct StatusDisplay {
    message: Option<String>,
    is_error: bool,
    timestamp: Option<Instant>,
    duration: Duration,
}

impl Default for StatusDisplay {
    fn default() -> Self {
        Self {
            message: None,
            is_error: false,
            timestamp: None,
            duration: Duration::from_secs(5),
        }
    }
}

impl StatusDisplay {
    pub fn set_message(&mut self, message: String, is_error: bool) {
        self.message = Some(message);
        self.is_error = is_error;
        self.timestamp = Some(Instant::now());
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.set_message(message.into(), true);
    }

    pub fn set_info(&mut self, message: impl Into<String>) {
        self.set_message(message.into(), false);
    }

    pub fn clear(&mut self) {
        self.message = None;
        self.timestamp = None;
    }

    pub fn show(&mut self, ui: &mut egui::Ui) {
        if let Some(message) = &self.message {
            if let Some(timestamp) = self.timestamp {
                if timestamp.elapsed() > self.duration {
                    self.message = None;
                    self.timestamp = None;
                    return;
                }
            }

            let color = if self.is_error {
                egui::Color32::RED
            } else {
                egui::Color32::GREEN
            };
            ui.colored_label(color, message);
        }
    }
}
