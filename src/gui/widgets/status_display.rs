use eframe::egui;

pub struct StatusDisplay {
    message: Option<String>,
    is_error: bool,
}

impl Default for StatusDisplay {
    fn default() -> Self {
        Self {
            message: None,
            is_error: false,
        }
    }
}

impl StatusDisplay {
    pub fn set_message(&mut self, message: impl Into<String>, is_error: bool) {
        self.message = Some(message.into());
        self.is_error = is_error;
    }

    pub fn set_error(&mut self, message: impl Into<String>) {
        self.set_message(message, true);
    }

    pub fn clear(&mut self) {
        self.message = None;
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        if let Some(message) = &self.message {
            let color = if self.is_error {
                ui.style().visuals.error_fg_color
            } else {
                ui.style().visuals.text_color()
            };
            ui.colored_label(color, message);
        }
    }
}
