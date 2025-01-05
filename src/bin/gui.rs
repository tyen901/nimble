#![allow(dead_code)]

use eframe::egui;
use egui::ViewportBuilder;
use nimble::gui::panels::{server_panel::ServerPanel, gen_srf_panel::GenSrfPanel};
use nimble::gui::state::{GuiState, GuiConfig, CommandMessage, CommandChannels};

#[derive(Default)]
struct NimbleGui {
    config: GuiConfig,
    state: GuiState,
    server_panel: ServerPanel,
    gen_srf_panel: GenSrfPanel,
    channels: CommandChannels,
    selected_tab: Tab,
}

#[derive(Default, PartialEq)]
enum Tab {
    #[default]
    Server,
    GenSrf,
}

impl NimbleGui {
    fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = GuiConfig::load();
        let server_panel = ServerPanel::from_config(&config);
        
        Self {
            config,
            server_panel,
            state: GuiState::default(),
            gen_srf_panel: GenSrfPanel::default(),
            channels: CommandChannels::default(),
            selected_tab: Tab::default(),
        }
    }
}

impl eframe::App for NimbleGui {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        // Update config from panels before saving
        self.config.repo_url = self.server_panel.repo_url().to_string();
        self.config.base_path = self.server_panel.base_path();
        
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
                ui.selectable_value(&mut self.selected_tab, Tab::Server, "Server");
                ui.selectable_value(&mut self.selected_tab, Tab::GenSrf, "Generate SRF");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Server => self.server_panel.show(ui, &self.state, Some(&self.channels.sender)),
                Tab::GenSrf => self.gen_srf_panel.show(ui, Some(&self.channels.sender), &self.state),
            }
            
            while let Ok(msg) = self.channels.receiver.try_recv() {
                match msg {
                    CommandMessage::ConfigChanged => {
                        // Update config when panels report changes
                        self.config.repo_url = self.server_panel.repo_url().to_string();
                        self.config.base_path = self.server_panel.base_path();
                        if let Err(e) = self.config.save() {
                            eprintln!("Failed to save config: {}", e);
                        }
                    }
                    CommandMessage::ConnectionStarted => {
                        self.state = GuiState::Connecting;
                    }
                    CommandMessage::ConnectionComplete(repo) => {
                        self.server_panel.set_repository(repo);
                        self.state = GuiState::Idle;
                    }
                    CommandMessage::ConnectionError(error) => {
                        println!("Connection error: {}", error);
                        self.state = GuiState::Idle;
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
                    CommandMessage::SyncCancelled => {
                        self.state = GuiState::Idle;
                    },
                    CommandMessage::CancelSync => {
                        // State will be updated when SyncCancelled is received
                    },
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
                }
            }
        });

        egui::TopBottomPanel::bottom("footer").show(ctx, |ui| {
            ui.horizontal(|ui| {
                match self.state {
                    GuiState::Idle => {
                        ui.label("Ready");
                    },
                    GuiState::Connecting => {
                        ui.label("Connecting...");
                    },
                    GuiState::GeneratingSRF { progress, .. } => {
                        ui.label("Generating SRF...");
                        ui.add(egui::ProgressBar::new(progress));
                    },
                    GuiState::Syncing { .. } => {
                        ui.label("Syncing...");
                    },
                    GuiState::Launching => {
                        ui.label("Launching...");
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
