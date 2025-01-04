use eframe::egui;
use std::path::PathBuf;
use rfd::FileDialog;

pub struct PathPicker {
    pub path: String,
    pub label: String,
    pub dialog_title: String,
}

impl PathPicker {
    pub fn new(label: impl Into<String>, dialog_title: impl Into<String>) -> Self {
        Self {
            path: String::new(),
            label: label.into(),
            dialog_title: dialog_title.into(),
        }
    }

    pub fn show(&mut self, ui: &mut egui::Ui) -> Option<PathBuf> {
        let mut selected = None;
        ui.horizontal(|ui| {
            ui.label(&self.label);
            ui.text_edit_singleline(&mut self.path);
            if ui.button("Browse").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title(&self.dialog_title)
                    .pick_folder() {
                        self.path = path.display().to_string();
                        selected = Some(path);
                }
            }
        });
        selected
    }

    pub fn path(&self) -> PathBuf {
        PathBuf::from(&self.path)
    }

    pub fn set_path(&mut self, path: &PathBuf) {
        self.path = path.display().to_string();
    }
}
