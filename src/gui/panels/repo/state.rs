use crate::gui::widgets::StatusDisplay;
use crate::repository::Repository;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use super::profile::ProfileManager;

// Make ConnectionState public
#[derive(PartialEq)]
pub enum ConnectionState {
    Disconnected,
    Connecting,
    Connected,
    Error(String),
}

pub struct RepoPanelState {
    pub(crate) status: StatusDisplay,
    pub(crate) repository: Option<Repository>,
    pub(crate) sync_cancel: Arc<AtomicBool>,
    pub(crate) scan_results: Option<Vec<crate::commands::scan::ModUpdate>>,
    pub(crate) profile_manager: ProfileManager,
    pub(crate) connection_state: ConnectionState,
}

impl Default for RepoPanelState {
    fn default() -> Self {
        Self {
            status: StatusDisplay::default(),
            repository: None,
            sync_cancel: Arc::new(AtomicBool::new(false)),
            scan_results: None,
            profile_manager: ProfileManager::default(),
            connection_state: ConnectionState::Disconnected,
        }
    }
}

impl RepoPanelState {
    pub fn status(&mut self) -> &mut StatusDisplay {
        &mut self.status
    }

    pub fn profile_manager(&mut self) -> &mut ProfileManager {
        &mut self.profile_manager
    }

    pub fn repository(&self) -> Option<&Repository> {
        self.repository.as_ref()
    }

    pub fn sync_cancel(&self) -> &Arc<AtomicBool> {
        &self.sync_cancel
    }

    pub fn set_repository(&mut self, repo: Repository) {
        self.repository = Some(repo);
        self.connection_state = ConnectionState::Connected;
    }

    pub fn set_connecting(&mut self) {
        self.connection_state = ConnectionState::Connecting;
    }

    pub fn set_connected(&mut self, repo: Repository) {
        self.repository = Some(repo);
        self.connection_state = ConnectionState::Connected;
    }

    pub fn set_connection_error(&mut self, error: String) {
        self.repository = None;
        self.connection_state = ConnectionState::Error(error);
    }

    pub fn disconnect(&mut self) {
        self.repository = None;
        self.connection_state = ConnectionState::Disconnected;
    }

    pub fn get_repository_info(&self) -> Option<(&str, &str, usize, usize)> {
        self.repository.as_ref().map(|repo| (
            repo.repo_name.as_str(),
            repo.version.as_str(),
            repo.required_mods.len(),
            repo.optional_mods.len()
        ))
    }

    pub fn is_connected(&self) -> bool {
        matches!(self.connection_state, ConnectionState::Connected)
    }

    pub fn connection_state(&self) -> &ConnectionState {
        &self.connection_state
    }

    pub fn set_scan_results(&mut self, results: Option<Vec<crate::commands::scan::ModUpdate>>) {
        self.scan_results = results;
    }
}
