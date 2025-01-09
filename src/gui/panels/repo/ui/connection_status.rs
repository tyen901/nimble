use eframe::egui;
use std::sync::mpsc::Sender;
use crate::gui::state::CommandMessage;
use super::super::state::{RepoPanelState, ConnectionState};
use super::super::connection::{connect_to_server, disconnect};

pub struct ConnectionStatusView;

impl ConnectionStatusView {
    pub fn show(ui: &mut egui::Ui, state: &mut RepoPanelState, sender: Option<&Sender<CommandMessage>>) {
        ui.horizontal(|ui| {
            Self::show_status_indicator(ui, state);
            Self::show_connection_button(ui, state, sender);
        });
    }

    fn show_status_indicator(ui: &mut egui::Ui, state: &RepoPanelState) {
        match state.connection_state() {
            ConnectionState::Connected => ui.label("🟢 Connected"),
            ConnectionState::Connecting => {
                let resp = ui.spinner();
                ui.label("Connecting...");
                resp
            },
            ConnectionState::Error(_) => ui.label("❌ Not Connected"),
            ConnectionState::Disconnected => {
                if state.is_offline_mode() {
                    ui.label("📴 Offline Mode")
                } else {
                    ui.label("❌ Not Connected")
                }
            },
        };
    }

    fn show_connection_button(ui: &mut egui::Ui, state: &mut RepoPanelState, sender: Option<&Sender<CommandMessage>>) {
        if let Some(sender) = sender {
            match state.connection_state() {
                ConnectionState::Connected => {
                    if ui.button("Disconnect").clicked() {
                        disconnect(state, sender);
                    }
                },
                ConnectionState::Disconnected | ConnectionState::Error(_) => {
                    if let Some(url) = state.profile_manager.get_current_url() {
                        if ui.button("Connect").clicked() {
                            connect_to_server(state, &url, sender);
                        }
                    }
                },
                _ => {}
            }
        }
    }
}
