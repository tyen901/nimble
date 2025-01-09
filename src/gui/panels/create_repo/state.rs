use std::path::PathBuf;
use crate::repository::{Repository, Mod};
use crate::gui::widgets::{PathPicker, StatusDisplay};
use crate::md5_digest::Md5Digest;

pub struct CreateRepoPanelState {
    pub repo: Repository,
    pub base_path: PathPicker,
    pub status: StatusDisplay,
    pub last_scanned_path: Option<PathBuf>,
    pub show_update_prompt: bool,
    pub pending_mods: Option<Vec<Mod>>,
    pub clean_options: CleanOptions,
}

pub struct CleanOptions {
    pub force_lowercase: bool,
    pub excluded_files: String,
    pub cleanup_files: bool,  // renamed from cleanup_enabled
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
            clean_options: CleanOptions {
                force_lowercase: true,
                excluded_files: ".git;.gitignore;.gitattributes;.gitmodules;.DS_Store;Thumbs.db;desktop.ini".to_string(),
                cleanup_files: true,
            },
        }
    }
}
