#![allow(dead_code)]

use eframe::egui;
use egui::ViewportBuilder;
use nimble::gui::panels::{sync_panel::SyncPanel, launch_panel::LaunchPanel, gen_srf_panel::GenSrfPanel};
use nimble::gui::state::{GuiState, GuiConfig, CommandMessage, CommandChannels};

#[derive(Default)]
struct NimbleGui {
    config: GuiConfig,
    state: GuiState,
    sync_panel: SyncPanel,
    launch_panel: LaunchPanel,
    gen_srf_panel: GenSrfPanel,
    channels: CommandChannels,
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
    fn new(cc: &eframe::CreationContext<'_>) -> Self {
        // Load config first
        let config = GuiConfig::load();
        
        // Initialize panels with config
        let sync_panel = SyncPanel::from_config(&config);
        
        Self {
            config,
            sync_panel,
            state: GuiState::default(),
            launch_panel: LaunchPanel::default(),
            gen_srf_panel: GenSrfPanel::default(),
            channels: CommandChannels::default(),
            selected_tab: Tab::default(),
        }
    }
}

impl eframe::App for NimbleGui {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // Update config from panels before saving
        self.config.repo_url = self.sync_panel.repo_url().to_string();
        self.config.base_path = self.sync_panel.base_path();
        
        if let Err(e) = self.config.save() {
            eprintln!("Failed to save config: {}", e);
        }
    }

    fn update(&mut self, ctx: &egui::Context, frame: &mut eframe::Frame) {
        // Update window size in config
        // TODO: Implement window size change handling

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
                Tab::Sync => self.sync_panel.show(ui, &self.state, Some(&self.channels.sender)),
                Tab::Launch => self.launch_panel.show(ui, &self.state, Some(&self.channels.sender)),
                Tab::GenSrf => self.gen_srf_panel.show(ui, &self.state, Some(&self.channels.sender)),
            }
            
            // Update state based on command messages
            while let Ok(msg) = self.channels.receiver.try_recv() {
                match msg {
                    CommandMessage::ConfigChanged => {
                        self.sync_panel.update_config(&mut self.config);
                        if let Err(e) = self.config.save() {
                            eprintln!("Failed to save config: {}", e);
                        }
                    }
                    CommandMessage::SyncProgress { file, progress, processed, total } => {
                        self.state = GuiState::Syncing { 
                            progress,
                            current_file: file,
                            files_processed: processed,
                            total_files: total,
                        };
                    }
                    CommandMessage::SyncComplete => {
                        self.state = GuiState::Idle;
                    }
                    CommandMessage::SyncError(error) => {
                        println!("Sync error: {}", error);
                        self.state = GuiState::Idle;
                    }
                    CommandMessage::GenSrfProgress { current_mod, progress, processed, total } => {
                        self.state = GuiState::GeneratingSRF {
                            progress,
                            current_mod,
                            mods_processed: processed,
                            total_mods: total,
                        };
                    }
                    CommandMessage::GenSrfComplete => {
                        self.state = GuiState::Idle;
                    }
                    CommandMessage::GenSrfError(error) => {
                        println!("GenSRF error: {}", error);
                        self.state = GuiState::Idle;
                    }
                    CommandMessage::LaunchStarted => {
                        self.state = GuiState::Launching;
                    }
                    CommandMessage::LaunchComplete => {
                        self.state = GuiState::Idle;
                    }
                    CommandMessage::LaunchError(error) => {
                        println!("Launch error: {}", error);
                        self.state = GuiState::Idle;
                    }
                }
            }
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.horizontal(|ui| {
                match self.state {
                    GuiState::Idle => {
                        ui.label("Ready");
                    },
                    GuiState::Syncing { progress, .. } => {
                        ui.label("Syncing...");
                        ui.add(egui::ProgressBar::new(progress));
                    },
                    GuiState::Launching => {
                        ui.label("Launching game...");
                    },
                    GuiState::GeneratingSRF { progress, .. } => {
                        ui.label("Generating SRF...");
                        ui.add(egui::ProgressBar::new(progress));
                    },
                }
            });
        });
    }
}

fn main() -> Result<(), eframe::Error> {
    let config = GuiConfig::load();
    let options = eframe::NativeOptions {
        viewport: ViewportBuilder::default()
            .with_inner_size(config.window_size()),
        ..Default::default()
    };
    
    eframe::run_native(
        "Nimble",
        options,
        Box::new(|cc| Ok(Box::new(NimbleGui::new(cc))))
    )
}
