use eframe::egui;
use egui::ViewportBuilder;
use crate::{repository, srf, config::Config, commands::sync::ProgressReporter};
use std::sync::{Arc, Mutex};
use std::collections::HashMap;
use std::time::Instant;

struct SyncProgress {
    current_stage: String,
    total_files: usize,
    total_repo_size: u64,    // Add total repo size
    total_download_size: u64, // Rename old total_size to be more specific
    tasks: HashMap<String, TaskProgress>,
    completed_files: Vec<String>,
    total_bytes_downloaded: u64,
    last_update: Instant,
    last_bytes_downloaded: u64,
    speed_samples: Vec<(Instant, u64)>,  // Store (timestamp, bytes) samples
    sample_window: std::time::Duration,   // How long to keep samples for
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

    fn set_total_files(&self, count: usize, download_size: u64, repo_size: u64) {
        let mut progress = self.lock().unwrap();
        progress.total_files = count;
        progress.total_download_size = download_size;
        progress.total_repo_size = repo_size;
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
        
        // First get all values we need
        let current_bytes = progress.tasks.get(filename).map(|t| t.bytes).unwrap_or(0);
        let bytes_delta = bytes.saturating_sub(current_bytes);
        let total_bytes = progress.total_bytes_downloaded + bytes_delta;
        let now = Instant::now();

        // Do all updates at once
        progress.total_bytes_downloaded = total_bytes;
        progress.speed_samples.push((now, total_bytes));
        
        // Clean up old samples
        let cutoff = now - progress.sample_window;
        progress.speed_samples.retain(|(t, _)| *t >= cutoff);

        // Update task
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

impl Default for SyncProgress {
    fn default() -> Self {
        Self {
            current_stage: String::new(),
            total_files: 0,
            total_repo_size: 0,
            total_download_size: 0,
            tasks: HashMap::new(),
            completed_files: Vec::new(),
            total_bytes_downloaded: 0,
            last_update: Instant::now(),
            last_bytes_downloaded: 0,
            speed_samples: Vec::with_capacity(100),
            sample_window: std::time::Duration::from_secs(5),
        }
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

fn format_size(bytes: u64) -> String {
    const UNITS: &[&str] = &["B", "KB", "MB", "GB", "TB"];
    let mut size = bytes as f64;
    let mut unit_index = 0;

    while size >= 1024.0 && unit_index < UNITS.len() - 1 {
        size /= 1024.0;
        unit_index += 1;
    }

    if unit_index == 0 {
        format!("{} {}", size as u64, UNITS[unit_index])
    } else {
        format!("{:.2} {}", size, UNITS[unit_index])
    }
}

fn format_duration(seconds: f64) -> String {
    if seconds < 60.0 {
        format!("{:.0} seconds", seconds)
    } else if seconds < 3600.0 {
        format!("{:.1} minutes", seconds / 60.0)
    } else {
        format!("{:.1} hours", seconds / 3600.0)
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

            ui.horizontal(|ui| {
                ui.label("Download Threads:");
                if ui.add(egui::DragValue::new(&mut self.config.download_threads)
                    .clamp_range(1..=32)
                ).changed() {
                    self.config.save().ok();
                }
            });

            if ui.button("Synchronize").clicked() {
                let local_path = self.config.local_path.clone();
                let repo_url = self.config.repo_url.clone();
                let mut agent = self.agent.clone();
                let progress = Arc::new(Mutex::new(SyncProgress::default()));
                self.sync_progress = Some(progress.clone());
                let threads = self.config.download_threads;
                
                std::thread::spawn(move || {
                    let path = std::path::Path::new(&local_path);
                    let _ = crate::commands::sync::sync(&mut agent, &repo_url, path, false, &progress, threads);
                });
            }

            // Show sync progress if available
            if let Some(progress) = &self.sync_progress {
                ui.separator();
                let progress = progress.lock().unwrap();
                
                ui.heading(&progress.current_stage);
                
                if progress.total_files > 0 {
                    ui.vertical(|ui| {
                        ui.label(format!(
                            "Files: {}/{}",
                            progress.completed_files.len(),
                            progress.total_files,
                        ));
                        ui.label(format!(
                            "Repository size: {}",
                            format_size(progress.total_repo_size)
                        ));
                        ui.label(format!(
                            "Download size: {}",
                            format_size(progress.total_download_size)
                        ));
                        
                        // Calculate overall progress and time estimate
                        let bytes_downloaded = progress.total_bytes_downloaded;
                        let total_size = progress.total_download_size;
                        
                        if bytes_downloaded > 0 {
                            // Calculate overall progress
                            let progress_frac = bytes_downloaded as f32 / total_size as f32;
                            
                            // Add overall progress bar
                            ui.add(egui::ProgressBar::new(progress_frac)
                                .show_percentage()
                                .animate(true));

                            // Calculate smooth speed from samples
                            let (speed, eta) = if progress.speed_samples.len() >= 2 {
                                let (oldest_time, oldest_bytes) = progress.speed_samples.first().unwrap();
                                let (latest_time, latest_bytes) = progress.speed_samples.last().unwrap();
                                
                                let elapsed = latest_time.duration_since(*oldest_time).as_secs_f64();
                                let bytes_delta = latest_bytes - oldest_bytes;
                                
                                let speed = bytes_delta as f64 / elapsed;
                                let remaining_bytes = total_size - bytes_downloaded;
                                let eta = remaining_bytes as f64 / speed;
                                
                                (speed, eta)
                            } else {
                                (0.0, 0.0)
                            };

                            ui.label(format!(
                                "Overall progress: {} / {}",
                                format_size(bytes_downloaded),
                                format_size(total_size),
                            ));
                            
                            ui.label(format!(
                                "Average speed: {:.1} MB/s",
                                speed / 1_000_000.0
                            ));
                            
                            if speed > 0.0 {
                                ui.label(format!(
                                    "Estimated time remaining: {}",
                                    format_duration(eta)
                                ));
                            }
                        }
                    });
                }
                
                for (filename, task) in &progress.tasks {
                    ui.group(|ui| {
                        ui.label(filename);
                        let progress_frac = task.bytes as f32 / task.total as f32;
                        ui.add(egui::ProgressBar::new(progress_frac));
                        ui.label(format!(
                            "{}/{} @ {:.1} MB/s",
                            format_size(task.bytes),
                            format_size(task.total),
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
