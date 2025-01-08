use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};
use eframe::egui;
use crate::repository::Repository;

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
    GenSrfProgress { current_mod: String, progress: f32, processed: usize, total: usize },
    GenSrfComplete,
    GenSrfError(String),
    LaunchStarted,
    LaunchComplete,
    LaunchError(String),
    Disconnect,
    ScanningStatus(String),
    ScanStarted,
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
    Connecting,
    Syncing { 
        progress: f32,
        current_file: String,
        files_processed: usize,
        total_files: usize,
    },
    Launching,
    GeneratingSRF { 
        progress: f32,
        current_mod: String,
        mods_processed: usize,
        total_mods: usize,
    },
    Scanning {
        message: String,
    },
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub repo_url: String,
    pub base_path: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
    #[serde(default = "default_version")]
    version: u32,
    #[serde(default = "default_window_size")]
    window_size: (f32, f32),
    #[serde(default)]
    pub profiles: Vec<Profile>,
    #[serde(default)]
    pub selected_profile: Option<String>,
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

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: "Default".to_string(),
            repo_url: String::new(),
            base_path: PathBuf::new(),
        }
    }
}

impl GuiConfig {
    pub const CURRENT_VERSION: u32 = 1;

    // Add version accessor methods
    pub fn version(&self) -> u32 {
        self.version
    }

    pub fn set_version(&mut self, version: u32) {
        self.version = version;
    }

    pub fn load() -> Self {
        super::config::load_config().unwrap_or_default()
    }

    pub fn save(&self) -> Result<(), String> {
        super::config::save_config(self)
    }

    // Modify validate to only check non-version requirements
    pub fn validate(&self) -> Result<(), String> {
        if let Some(profile) = self.get_selected_profile() {
            if !profile.base_path.exists() {
                return Err("Base path does not exist".into());
            }
            if !profile.base_path.is_dir() {
                return Err("Base path is not a directory".into());
            }
            if profile.repo_url.is_empty() {
                return Err("Repository URL is required".into());
            }
        }
        Ok(())
    }

    pub fn window_size(&self) -> egui::Vec2 {
        egui::Vec2::new(self.window_size.0, self.window_size.1)
    }

    pub fn set_window_size(&mut self, size: egui::Vec2) {
        self.window_size = (size.x, size.y);
    }

    pub fn add_profile(&mut self, profile: Profile) {
        self.profiles.push(profile);
    }

    pub fn remove_profile(&mut self, name: &str) {
        self.profiles.retain(|p| p.name != name);
        if self.selected_profile.as_deref() == Some(name) {
            self.selected_profile = None;
        }
    }

    pub fn get_profile(&self, name: &str) -> Option<&Profile> {
        self.profiles.iter().find(|p| p.name == name)
    }

    pub fn get_selected_profile(&self) -> Option<&Profile> {
        self.selected_profile.as_ref().and_then(|name| self.get_profile(name))
    }
}
