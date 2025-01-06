use eframe::egui;
use std::sync::mpsc::Sender;
use crate::gui::state::{CommandMessage, GuiState};
use crate::gui::widgets::CommandHandler;
use super::{state::GenSrfPanelState, ui};

pub struct GenSrfPanel {
    state: GenSrfPanelState,
}

impl CommandHandler for GenSrfPanel {}

impl Default for GenSrfPanel {
    fn default() -> Self {
        Self {
            state: GenSrfPanelState::default(),
        }
    }
}

impl GenSrfPanel {
    pub fn show(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>, gui_state: &GuiState) {
        ui::render_panel(ui, &mut self.state, sender, gui_state);
    }
}
