use crate::gui::widgets::StatusDisplay;
use crate::repository::Repository;
use crate::mod_cache::ModCache;  // Add this import
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

#[derive(PartialEq)]
pub enum CacheState {
    NoCache,
    CacheLoaded(chrono::DateTime<chrono::Utc>),
    NeedsSync,
}

pub struct RepoPanelState {
    pub(crate) status: StatusDisplay,
    pub(crate) repository: Option<Repository>,
    pub(crate) sync_cancel: Arc<AtomicBool>,
    pub(crate) scan_results: Option<Vec<crate::commands::scan::ModUpdate>>,
    pub(crate) profile_manager: ProfileManager,
    pub(crate) connection_state: ConnectionState,
    pub(crate) is_offline_mode: bool,
    pub(crate) cache_state: CacheState,
    pub(crate) local_repository: Option<Repository>,  // From cache
    pub(crate) remote_repository: Option<Repository>, // From server
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
            is_offline_mode: false,
            cache_state: CacheState::NoCache,
            local_repository: None,
            remote_repository: None,
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
        // Don't clear repository data on error, only change connection state
        self.connection_state = ConnectionState::Error(error);
    }

    pub fn disconnect(&mut self) {
        // Don't clear repository data on disconnect, only change connection state
        self.connection_state = ConnectionState::Disconnected;
    }

    // Add method to explicitly clear repository data
    pub fn clear_repository(&mut self) {
        self.repository = None;
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

    pub fn set_offline_mode(&mut self, offline: bool) {
        self.is_offline_mode = offline;
    }

    pub fn is_offline_mode(&self) -> bool {
        self.is_offline_mode
    }

    pub fn has_repository_data(&self) -> bool {
        self.repository.is_some()
    }

    pub fn get_launch_parameters(&self) -> Option<String> {
        // Check local repository first, then remote
        self.local_repository
            .as_ref()
            .map(|repo| repo.client_parameters.clone())
    }

    pub fn load_cache(&mut self, cache: &ModCache) {
        self.local_repository = cache.repository.clone();
        self.cache_state = match cache.last_sync {
            Some(time) => CacheState::CacheLoaded(time),
            None => CacheState::NeedsSync,
        };
    }

    pub fn update_from_remote(&mut self, repo: Repository) {
        self.remote_repository = Some(repo.clone());
        self.local_repository = Some(repo);
        self.cache_state = CacheState::CacheLoaded(chrono::Utc::now());
    }

    pub fn has_local_data(&self) -> bool {
        self.local_repository.is_some()
    }

    pub fn get_repository_for_launch(&self) -> Option<&Repository> {
        self.local_repository.as_ref()
    }

    pub fn sync_age(&self) -> Option<chrono::Duration> {
        match &self.cache_state {
            CacheState::CacheLoaded(time) => Some(chrono::Utc::now() - *time),
            _ => None
        }
    }

    pub fn set_selected_profile(&mut self, profile_name: Option<String>) {
        self.profile_manager.set_selected(profile_name);
        
        // Load cache for the new profile
        if let Some(profile) = self.profile_manager.get_selected_profile() {
            if let Ok(cache) = ModCache::from_disk_or_empty(&profile.base_path) {
                self.load_cache(&cache);
            } else {
                // Clear local repository data if we can't load cache
                self.local_repository = None;
                self.cache_state = CacheState::NoCache;
            }
        } else {
            // Clear local repository data if no profile selected
            self.local_repository = None;
            self.cache_state = CacheState::NoCache;
        }
    }

    pub fn clear_local_data(&mut self) {
        self.local_repository = None;
        self.cache_state = CacheState::NoCache;
        self.is_offline_mode = false;
    }
}
