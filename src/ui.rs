use eframe::egui;
use egui::ViewportBuilder;
use crate::{repository, srf, config::Config};

pub struct NimbleApp {
    repository: Option<repository::Repository>,
    mods: Vec<srf::Mod>,
    error: Option<String>,
    config: Config,
    agent: ureq::Agent,
}

impl Default for NimbleApp {
    fn default() -> Self {
        Self {
            repository: None,
            mods: Vec::new(),
            error: None,
            config: Config::load(),
            agent: ureq::AgentBuilder::new()
                .build(),
        }
    }
}

impl eframe::App for NimbleApp {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::CentralPanel::default().show(ctx, |ui| {
            ui.heading("Nimble Mod Manager");

            if let Some(error) = &self.error {
                ui.colored_label(egui::Color32::RED, error);
                ui.separator();
            }

            ui.horizontal(|ui| {
                ui.label("Repository URL:");
                if ui.text_edit_singleline(&mut self.config.repo_url).changed() {
                    self.config.save().ok();
                }
            });
            ui.horizontal(|ui| {
                ui.label("Local Path:");
                ui.text_edit_singleline(&mut self.config.local_path);
                if ui.button("Browse...").clicked() {
                    if let Some(path) = rfd::FileDialog::new().pick_folder() {
                        self.config.local_path = path.display().to_string();
                        self.config.save().ok();
                    }
                }
            });

            if ui.button("Synchronize").clicked() {
                let path = std::path::Path::new(&self.config.local_path);
                match crate::commands::sync::sync(&mut self.agent, &self.config.repo_url, path, false) {
                    Ok(_) => {
                        self.error = None;
                    }
                    Err(e) => {
                        self.error = Some(format!("Sync failed: {}", e));
                    }
                }
            }

            ui.separator();

            ui.collapsing("Local Mods", |ui| {
                for mod_entry in &self.mods {
                    ui.label(&mod_entry.name);
                    ui.label(format!("Checksum: {}", hex::encode(mod_entry.checksum.as_bytes())));
                    ui.separator();
                }
            });
        });
    }
}

pub fn run_ui() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size([800.0, 600.0]),
        ..Default::default()
    };

    eframe::run_native(
        "Nimble",
        options,
        Box::new(|_cc| Ok(Box::new(NimbleApp::default())))
    )
}
