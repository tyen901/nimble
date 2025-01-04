#![allow(dead_code)]

use eframe::egui;
use nimble::gui::panels::{sync_panel::SyncPanel, launch_panel::LaunchPanel};
use nimble::gui::state::{GuiState, GuiConfig};

#[derive(Default)]
struct NimbleGui {
    config: GuiConfig,
    state: GuiState,
    sync_panel: SyncPanel,
    launch_panel: LaunchPanel,
    selected_tab: Tab,
}

#[derive(Default, PartialEq)]
enum Tab {
    #[default]
    Sync,
    Launch,
    GenSrf,
}

impl NimbleGui {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        Self::default()
    }
}

impl eframe::App for NimbleGui {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Nimble");
                ui.separator();
                ui.selectable_value(&mut self.selected_tab, Tab::Sync, "Sync");
                ui.selectable_value(&mut self.selected_tab, Tab::Launch, "Launch");
                ui.selectable_value(&mut self.selected_tab, Tab::GenSrf, "Generate SRF");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Sync => self.sync_panel.show(ui),
                Tab::Launch => self.launch_panel.show(ui),
                Tab::GenSrf => {
                    ui.heading("Generate SRF");
                    // TODO: Implement SRF panel
                },
            }
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.horizontal(|ui| {
                match self.state {
                    GuiState::Idle => {
                        ui.label("Ready");
                    },
                    GuiState::Syncing { progress } => {
                        ui.label("Syncing...");
                        ui.add(egui::ProgressBar::new(progress));
                    },
                    GuiState::Launching => {
                        ui.label("Launching game...");
                    },
                    GuiState::GeneratingSRF { progress } => {
                        ui.label("Generating SRF...");
                        ui.add(egui::ProgressBar::new(progress));
                    },
                }
            });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        initial_window_size: Some(egui::vec2(800.0, 600.0)),
        ..Default::default()
    };
    
    eframe::run_native(
        "Nimble",
        options,
        Box::new(|cc| Box::new(NimbleGui::new(cc)))
    )
}
