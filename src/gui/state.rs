use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Debug, Clone, PartialEq)]
pub enum GuiState {
    Idle,
    Syncing { progress: f32 },
    Launching,
    GeneratingSRF { progress: f32 },
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
