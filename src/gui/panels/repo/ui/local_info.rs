use eframe::egui;
use chrono::Duration;
use super::super::state::RepoPanelState;
use super::repository_info::format_repository_info;
use std::process::Command;

pub struct LocalInfoView;

impl LocalInfoView {
    pub fn show(ui: &mut egui::Ui, state: &mut RepoPanelState) {
        // Get all required data before ui.group to avoid borrow issues
        let profile = state.profile_manager().get_selected_profile().cloned();
        let sync_age = state.sync_age();
        let repo = state.get_repository_for_launch().cloned();

        ui.group(|ui| {
            ui.heading("Local Cache");
            
            // Show installation path with explorer button
            if let Some(profile) = profile {
                ui.horizontal(|ui| {
                    ui.strong("Install Path:");
                    ui.label(profile.base_path.to_string_lossy().to_string());
                    if ui.small_button("📂").clicked() && profile.base_path.exists() {
                        Self::open_in_explorer(&profile.base_path);
                    }
                });
            }

            // Show sync age
            if let Some(age) = sync_age {
                ui.horizontal(|ui| {
                    ui.label("Last Synced:");
                    ui.label(format_duration(age));
                });
            }

            // Show repository info
            if let Some(repo) = repo {
                ui.add_space(4.0);
                format_repository_info(ui, &repo);
            }
        });
    }

    #[cfg(target_os = "windows")]
    fn open_in_explorer(path: &std::path::Path) {
        if path.exists() {
            Command::new("explorer")
                .arg(path.as_os_str())
                .spawn()
                .ok();
        }
    }

    #[cfg(target_os = "linux")]
    fn open_in_explorer(path: &std::path::Path) {
        if path.exists() {
            Command::new("xdg-open")
                .arg(path.as_os_str())
                .spawn()
                .ok();
        }
    }

    #[cfg(target_os = "macos")]
    fn open_in_explorer(path: &std::path::Path) {
        if path.exists() {
            Command::new("open")
                .arg(path.as_os_str())
                .spawn()
                .ok();
        }
    }
}

fn format_duration(duration: Duration) -> String {
    if duration.num_days() > 0 {
        format!("{} days ago", duration.num_days())
    } else if duration.num_hours() > 0 {
        format!("{} hours ago", duration.num_hours())
    } else if duration.num_minutes() > 0 {
        format!("{} minutes ago", duration.num_minutes())
    } else {
        "just now".to_string()
    }
}
