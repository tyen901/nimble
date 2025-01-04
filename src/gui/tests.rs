#[cfg(test)]
mod tests {
    use super::super::state::{GuiState, GuiConfig};
    use super::super::panels::{sync_panel::SyncPanel, launch_panel::LaunchPanel};

    #[test]
    fn test_gui_state_defaults() {
        let state = GuiState::Idle;
        assert!(matches!(state, GuiState::Idle));
    }

    #[test]
    fn test_sync_panel() {
        let panel = SyncPanel::default();
        assert!(panel.repo_url.is_empty());
        assert!(panel.base_path.is_empty());
    }

    #[test]
    fn test_launch_panel() {
        let panel = LaunchPanel::default();
        assert!(panel.base_path.is_empty());
    }
}
