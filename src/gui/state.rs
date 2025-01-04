use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::mpsc::{channel, Receiver, Sender};

#[derive(Debug)]
pub enum CommandMessage {
    SyncProgress { file: String, progress: f32, processed: usize, total: usize },
    SyncComplete,
    SyncError(String),
    GenSrfProgress { current_mod: String, progress: f32, processed: usize, total: usize },
    GenSrfComplete,
    GenSrfError(String),
    LaunchStarted,
    LaunchComplete,
    LaunchError(String),
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

#[derive(Debug, Clone, PartialEq)]
pub enum GuiState {
    Idle,
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
}

impl Default for GuiState {
    fn default() -> Self {
        Self::Idle
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuiConfig {
    pub repo_url: String,
    pub base_path: PathBuf,
    pub window_size: (f32, f32),
}

impl Default for GuiConfig {
    fn default() -> Self {
        Self {
            repo_url: String::new(),
            base_path: PathBuf::new(),
            window_size: (800.0, 600.0),
        }
    }
}

impl GuiConfig {
    pub fn load() -> Self {
        if let Ok(config_str) = std::fs::read_to_string("nimble_config.json") {
            serde_json::from_str(&config_str).unwrap_or_default()
        } else {
            Self::default()
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_str = serde_json::to_string_pretty(self)?;
        std::fs::write("nimble_config.json", config_str)
    }

    pub fn validate(&self) -> Result<(), String> {
        if !self.base_path.exists() {
            return Err("Base path does not exist".into());
        }
        if !self.base_path.is_dir() {
            return Err("Base path is not a directory".into());
        }
        if self.repo_url.is_empty() {
            return Err("Repository URL is required".into());
        }
        Ok(())
    }
}
