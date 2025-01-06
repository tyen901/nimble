use std::path::PathBuf;
use crate::repository::{Repository, Mod};
use crate::gui::widgets::{PathPicker, StatusDisplay};
use crate::gui::state::GuiConfig;

pub struct CreateRepoPanelState {
    pub repo: Repository,
    pub base_path: PathPicker,
    pub status: StatusDisplay,
    pub last_scanned_path: Option<PathBuf>,
    pub auto_increment_version: bool,
    pub show_update_prompt: bool,
    pub pending_mods: Option<Vec<Mod>>,
    pub config: Option<GuiConfig>,
}

impl Default for CreateRepoPanelState {
    fn default() -> Self {
        Self {
            repo: Repository {
                repo_name: String::new(),
                checksum: String::new(),
                required_mods: Vec::new(),
                optional_mods: Vec::new(),
                client_parameters: "-noPause -noSplash -skipIntro".to_string(),
                repo_basic_authentication: None,
                version: "1.0.0".to_string(),
                servers: Vec::new(),
            },
            base_path: PathPicker::new("Repository Path:", "Select Repository Directory"),
            status: StatusDisplay::default(),
            last_scanned_path: None,
            auto_increment_version: true,
            show_update_prompt: false,
            pending_mods: None,
            config: None,
        }
    }
}
