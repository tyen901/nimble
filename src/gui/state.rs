use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use eframe::egui;
use crate::repository::Repository;
use crate::gui::panels::repo::Profile;

#[derive(Debug)]
pub enum CommandMessage {
    ConfigChanged,
    ConnectionStarted,
    ConnectionComplete(Repository),
    ConnectionError(String),
    SyncProgress { file: String, progress: f32, processed: usize, total: usize },
    SyncComplete,
    SyncError(String),
    SyncCancelled,
    CancelSync,
    LaunchStarted,
    LaunchComplete,
    LaunchError(String),
    Disconnect,
    ScanningStatus(String),
    ScanStarted,
    ScanComplete(Vec<crate::commands::scan::ModUpdate>),
    SyncStarted,
}

pub struct CommandChannels {
    pub sender: Sender<CommandMessage>,
    pub receiver: Receiver<CommandMessage>,
}

impl CommandChannels {
    pub fn new() -> Self {
        let (sender, receiver) = channel();
        Self { sender, receiver }
    }
}

impl Default for CommandChannels {
    fn default() -> Self {
        Self::new()
    }
}

#[derive(Debug, Clone, PartialEq, Default)]
pub enum GuiState {
    #[default]
    Idle,
    Syncing { 
        progress: f32,
        current_file: String,
        files_processed: usize,
        total_files: usize,
    },
    Launching,
    Scanning {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default = "default_window_size")]
    window_size: (f32, f32),
    #[serde(default)]
    profiles: Vec<Profile>,
    #[serde(default)]
    selected_profile: Option<String>,
}

fn default_version() -> u32 {
    1
}

fn default_window_size() -> (f32, f32) {
    (800.0, 600.0)
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            version: default_version(),
            window_size: default_window_size(),
            profiles: Vec::new(),
            selected_profile: None,
        }
    }
}

// Add custom error type
#[derive(Debug)]
pub enum ConfigError {
    IoError(std::io::Error),
    ParseError(serde_json::Error),
    VersionError(String),
    ValidationError(String),
}

impl std::fmt::Display for ConfigError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::IoError(e) => write!(f, "IO error: {}", e),
            Self::ParseError(e) => write!(f, "Parse error: {}", e),
            Self::VersionError(e) => write!(f, "Version error: {}", e),
            Self::ValidationError(e) => write!(f, "Validation error: {}", e),
        }
    }
}

impl GuiConfig {
    pub const CURRENT_VERSION: u32 = 1;

    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn set_version(&mut self, version: u32) {
        self.version = version;
    }

    pub fn load() -> Self {
        super::config::load_config().unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), ConfigError> {
        super::config::save_config(self)
            .map_err(|e| ConfigError::ValidationError(e))
    }

    pub fn validate(&self) -> Result<(), String> {
        Ok(())
    }

    pub fn window_size(&self) -> egui::Vec2 {
        egui::Vec2::new(self.window_size.0, self.window_size.1)
    }

    pub fn set_window_size(&mut self, size: egui::Vec2) {
        self.window_size = (size.x, size.y);
    }

    pub fn get_profiles(&self) -> &Vec<Profile> {
        &self.profiles
    }

    pub fn get_selected_profile_name(&self) -> &Option<String> {
        &self.selected_profile
    }

    pub fn set_profiles(&mut self, profiles: Vec<Profile>) {
        self.profiles = profiles;
    }

    pub fn set_selected_profile(&mut self, profile: Option<String>) {
        self.selected_profile = profile;
    }

    pub fn get_selected_profile(&self) -> Option<&Profile> {
        self.selected_profile
            .as_ref()
            .and_then(|name| self.profiles.iter().find(|p| &p.name == name))
    }
}
