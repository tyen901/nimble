pub mod widgets;
pub mod panels;
pub mod state;
pub mod config;

use eframe::egui;
use egui::ViewportBuilder;
use crate::gui::panels::{create_repo::CreateRepoPanel, repo::RepoPanel};
use crate::gui::state::{GuiState, GuiConfig, CommandMessage, CommandChannels};

#[derive(Default)]
pub struct NimbleGui {
    config: GuiConfig,
    state: GuiState,
    repo_panel: RepoPanel,
    create_repo_panel: CreateRepoPanel,
    channels: CommandChannels,
    selected_tab: Tab,
}

#[derive(Default, PartialEq)]
pub enum Tab {
    #[default]
    Server,
    CreateRepo,
}

impl NimbleGui {
    pub fn new(_cc: &eframe::CreationContext<'_>) -> Self {
        let config = GuiConfig::load();
        
        Self {
            config: config.clone(),
            state: GuiState::default(),
            repo_panel: RepoPanel::from_config(&config),
            create_repo_panel: CreateRepoPanel::default(),
            channels: CommandChannels::default(),
            selected_tab: Tab::default(),
        }
    }
}

impl eframe::App for NimbleGui {
    fn save(&mut self, _storage: &mut dyn eframe::Storage) {
        self.repo_panel.save_to_config(&mut self.config);
        if let Err(e) = self.config.save() {
            eprintln!("Failed to save config: {}", e);
        }
    }

    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        // Update window size in config
        // TODO: Implement window size change handling

        egui::TopBottomPanel::top("header").show(ctx, |ui| {
            ui.horizontal(|ui| {
                ui.heading("Nimble");
                ui.separator();
                ui.selectable_value(&mut self.selected_tab, Tab::Server, "Server");
                ui.selectable_value(&mut self.selected_tab, Tab::CreateRepo, "Create Repo");
            });
        });

        egui::CentralPanel::default().show(ctx, |ui| {
            match self.selected_tab {
                Tab::Server => self.repo_panel.show(ui, &self.state, Some(&self.channels.sender)),
                Tab::CreateRepo => self.create_repo_panel.show(ui),
            }
            
            while let Ok(msg) = self.channels.receiver.try_recv() {
                // First let the repo panel handle its own state
                self.repo_panel.handle_command(&msg);

                // Then handle global state changes
                match msg {
                    CommandMessage::ConfigChanged => {
                        self.repo_panel.save_to_config(&mut self.config);
                        self.config.save().unwrap_or_else(|e| eprintln!("Failed to save config: {}", e));
                    }
                    CommandMessage::SyncProgress { file, progress, processed, total } => {
                        self.state = GuiState::Syncing {
                            progress,
                            current_file: file,
                            files_processed: processed,
                            total_files: total,
                        };
                        ctx.request_repaint();
                    }
                    CommandMessage::LaunchStarted => self.state = GuiState::Launching,
                    CommandMessage::ScanningStatus(message) => {
                        self.state = GuiState::Scanning { message };
                        ctx.request_repaint();
                    }
                    CommandMessage::ScanStarted => {
                        self.state = GuiState::Scanning { 
                            message: "Scanning local folder...".into() 
                        };
                    }
                    // All these states just return to Idle
                    CommandMessage::SyncComplete |
                    CommandMessage::SyncError(_) |
                    CommandMessage::SyncCancelled |
                    CommandMessage::LaunchComplete |
                    CommandMessage::LaunchError(_) => self.state = GuiState::Idle,
                    // These are handled by the repo panel
                    CommandMessage::ConnectionStarted |
                    CommandMessage::ConnectionComplete(_) |
                    CommandMessage::ConnectionError(_) |
                    CommandMessage::Disconnect |
                    CommandMessage::CancelSync => {}
                }
            }
        });
    }
}
