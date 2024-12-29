use eframe::egui;
use egui::ViewportBuilder;
use crate::{repository, srf, config::Config, commands::sync::ProgressReporter};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;

#[derive(Default)]
struct SyncProgress {
    current_stage: String,
    total_files: usize,
    total_size: u64,
    tasks: HashMap<String, TaskProgress>,
    completed_files: Vec<String>,
}

#[derive(Default)]
struct TaskProgress {
    total: u64,
    bytes: u64,
    speed: f64,
}

impl ProgressReporter for Arc<Mutex<SyncProgress>> {
    fn set_stage(&self, stage: &str) {
        let mut progress = self.lock().unwrap();
        progress.current_stage = stage.to_string();
    }

    fn set_total_files(&self, count: usize, total_size: u64) {
        let mut progress = self.lock().unwrap();
        progress.total_files = count;
        progress.total_size = total_size;
    }

    fn start_task(&self, filename: &str, total: u64) {
        let mut progress = self.lock().unwrap();
        progress.tasks.insert(
            filename.to_string(),
            TaskProgress {
                total,
                bytes: 0,
                speed: 0.0,
            },
        );
    }

    fn update_file_progress(&self, filename: &str, bytes: u64, total: u64, speed: f64) {
        let mut progress = self.lock().unwrap();
        if let Some(task) = progress.tasks.get_mut(filename) {
            task.bytes = bytes;
            task.total = total;
            task.speed = speed;
        }
    }
        
    fn file_completed(&self, filename: &str) {
        let mut progress = self.lock().unwrap();
        progress.completed_files.push(filename.to_string());
        progress.tasks.remove(filename);
    }
}

pub struct NimbleApp {
    repository: Option<repository::Repository>,
    mods: Vec<srf::Mod>,
    error: Option<String>,
    config: Config,
    agent: ureq::Agent,
    sync_progress: Option<Arc<Mutex<SyncProgress>>>,
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
            sync_progress: None,
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
                let local_path = self.config.local_path.clone();
                let repo_url = self.config.repo_url.clone();
                let mut agent = self.agent.clone();
                let progress = Arc::new(Mutex::new(SyncProgress::default()));
                self.sync_progress = Some(progress.clone());
                
                std::thread::spawn(move || {
                    let path = std::path::Path::new(&local_path);
                    let _ = crate::commands::sync::sync(&mut agent, &repo_url, path, false, &progress);
                });
            }

            // Show sync progress if available
            if let Some(progress) = &self.sync_progress {
                ui.separator();
                let progress = progress.lock().unwrap();
                
                ui.heading(&progress.current_stage);
                
                if progress.total_files > 0 {
                    ui.label(format!(
                        "Files: {}/{} ({} bytes total)",
                        progress.completed_files.len(),
                        progress.total_files,
                        progress.total_size
                    ));
                }
                
                for (filename, task) in &progress.tasks {
                    ui.group(|ui| {
                        ui.label(filename);
                        let progress_frac = task.bytes as f32 / task.total as f32;
                        ui.add(egui::ProgressBar::new(progress_frac));
                        ui.label(format!(
                            "{}/{} @ {:.1} MB/s",
                            task.bytes,
                            task.total,
                            task.speed / 1_000_000.0
                        ));
                    });
                }
            }
        });

        // Request a redraw to ensure the UI updates
        ctx.request_repaint();
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
