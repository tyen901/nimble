use eframe::egui;

pub struct SyncPanel {
    repo_url: String,
    base_path: String,
}

impl Default for SyncPanel {
    fn default() -> Self {
        Self {
            repo_url: String::new(),
            base_path: String::new(),
        }
    }
}

impl SyncPanel {
    pub fn show(&mut self, ui: &mut egui::Ui) {
        ui.heading("Sync Mods");
        ui.add_space(8.0);
        
        ui.horizontal(|ui| {
            ui.label("Repository URL:");
            ui.text_edit_singleline(&mut self.repo_url);
        });
        
        ui.horizontal(|ui| {
            ui.label("Base Path:");
            ui.text_edit_singleline(&mut self.base_path);
            if ui.button("Browse").clicked() {
                // TODO: Implement file dialog
            }
        });
        
        if ui.button("Start Sync").clicked() {
            // TODO: Implement sync logic
        }
    }
}
