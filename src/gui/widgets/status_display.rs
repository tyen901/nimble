use eframe::egui;

pub struct StatusDisplay {
    error: Option<String>,
    status: Option<String>,
}

impl Default for StatusDisplay {
    fn default() -> Self {
        Self {
            error: None,
            status: None,
        }
    }
}

impl StatusDisplay {
    pub fn set_error(&mut self, error: impl Into<String>) {
        self.error = Some(error.into());
        self.status = None;
    }

    pub fn set_status(&mut self, status: impl Into<String>) {
        self.status = Some(status.into());
        self.error = None;
    }

    pub fn clear(&mut self) {
        self.error = None;
        self.status = None;
    }

    pub fn show(&self, ui: &mut egui::Ui) {
        if let Some(error) = &self.error {
            ui.colored_label(ui.style().visuals.error_fg_color, error);
            ui.add_space(8.0);
        }
        
        if let Some(status) = &self.status {
            ui.label(status);
            ui.add_space(8.0);
        }
    }
}
