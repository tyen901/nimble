use eframe::egui;

pub struct LaunchPanel {
    base_path: String,
}

impl Default for LaunchPanel {
    fn default() -> Self {
        Self {
            base_path: String::new(),
        }
    }
}

impl LaunchPanel {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("Launch Arma 3");
        ui.add_space(8.0);
        
        ui.horizontal(|ui| {
            ui.label("Mods Path:");
            ui.text_edit_singleline(&mut self.base_path);
            if ui.button("Browse").clicked() {
                // TODO: Implement file dialog
            }
        });
        
        if ui.button("Launch Game").clicked() {
            // TODO: Implement launch logic
        }
    }
}
