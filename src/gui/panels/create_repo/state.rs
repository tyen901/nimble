use std::path::PathBuf;
use crate::repository::{Repository, Mod};
use crate::gui::widgets::{PathPicker, StatusDisplay};
use crate::gui::state::GuiConfig;
use crate::md5_digest::Md5Digest;

pub struct CreateRepoPanelState {
    pub repo: Repository,
    pub base_path: PathPicker,
    pub status: StatusDisplay,
    pub last_scanned_path: Option<PathBuf>,
    pub show_update_prompt: bool,  // Removed auto_increment_version
    pub pending_mods: Option<Vec<Mod>>,
    pub config: Option<GuiConfig>,
    pub clean_options: CleanOptions,
}

pub struct CleanOptions {
    pub force_lowercase: bool,
    pub file_filters: Vec<String>,
    pub new_filter: String,
    pub auto_clean: bool,
}

impl Default for CreateRepoPanelState {
    fn default() -> Self {
        Self {
            repo: Repository {
                repo_name: String::new(),
                checksum: Md5Digest::default(),
                required_mods: Vec::new(),
                optional_mods: Vec::new(),
                client_parameters: "-noPause -noSplash -skipIntro".to_string(),
                repo_basic_authentication: None,
                version: "3.2.0.0".to_string(),  // Set fixed version
                servers: Vec::new(),
            },
            base_path: PathPicker::new("Repository Path:", "Select Repository Directory"),
            status: StatusDisplay::default(),
            last_scanned_path: None,
            show_update_prompt: false,
            pending_mods: None,
            config: None,
            clean_options: CleanOptions {
                force_lowercase: true,
                file_filters: vec![
                    ".git".to_string(),
                    ".gitignore".to_string(),
                    ".gitattributes".to_string(),
                    ".gitmodules".to_string(),
                    ".DS_Store".to_string(),
                    "Thumbs.db".to_string(),
                    "desktop.ini".to_string(),
                ],
                new_filter: String::new(),
                auto_clean: true,
            },
        }
    }
}
