use eframe::egui;
use std::path::PathBuf;

pub struct PathPicker {
    path: PathBuf,
    label: String,
    dialog_title: String,
}

impl PathPicker {
    pub fn new(label: &str, dialog_title: &str) -> Self {
        Self {
            path: PathBuf::new(),
            label: label.to_string(),
            dialog_title: dialog_title.to_string(),
        }
    }

    pub fn path(&self) -> PathBuf {
        self.path.clone()
    }

    pub fn set_path(&mut self, path: &PathBuf) {
        self.path = path.clone();
    }

    // Return true if the path was changed
    pub fn show(&mut self, ui: &mut egui::Ui) -> bool {
        let mut changed = false;
        ui.horizontal(|ui| {
            ui.label(&self.label);
            let path_string = self.path.to_string_lossy().to_string();
            let mut text = path_string.clone();
            if ui.text_edit_singleline(&mut text).changed() {
                self.path = PathBuf::from(text);
                changed = true;
            }
            if ui.button("Browse").clicked() {
                if let Some(path) = rfd::FileDialog::new()
                    .set_title(&self.dialog_title)
                    .pick_folder() 
                {
                    self.path = path;
                    changed = true;
                }
            }
        });
        changed
    }

    // Return true if the path was changed
    pub fn show_picker(&mut self) -> bool {
        if let Some(path) = rfd::FileDialog::new()
            .set_title(&self.dialog_title)
            .pick_folder() 
        {
            self.path = path;
            true
        } else {
            false
        }
    }
}
