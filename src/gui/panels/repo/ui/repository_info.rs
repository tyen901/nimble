use eframe::egui;
use crate::repository::Repository;

pub struct RepositoryInfoView;

impl RepositoryInfoView {
    pub fn show(ui: &mut egui::Ui, repo: &Repository, _base_path: Option<&std::path::PathBuf>) {
        ui.group(|ui| {
            ui.heading("Remote Repository");
            format_repository_info(ui, repo);
        });
    }
}

// Common function for formatting repository info
pub(crate) fn format_repository_info(ui: &mut egui::Ui, repo: &Repository) {
    ui.horizontal(|ui| {
        ui.strong("Name:");
        ui.label(&repo.repo_name);
    });
    ui.horizontal(|ui| {
        ui.strong("Version:");
        ui.label(&repo.version);
    });
    ui.horizontal(|ui| {
        ui.label(format!(
            "{} required and {} optional mods",
            repo.required_mods.len(),
            repo.optional_mods.len()
        ));
    });
}
