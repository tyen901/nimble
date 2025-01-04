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
