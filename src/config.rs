use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::fs;

#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    pub repo_url: String,
    pub local_path: String,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            repo_url: "http://swifty.peanutcommunityarma.com/".to_string(),
            local_path: String::new(),
        }
    }
}

impl Config {
    pub fn load() -> Self {
        let config_path = Config::get_config_path();
        if let Ok(contents) = fs::read_to_string(config_path) {
            serde_json::from_str(&contents).unwrap_or_default()
        } else {
            Config::default()
        }
    }

    pub fn save(&self) -> Result<(), std::io::Error> {
        let config_path = Config::get_config_path();
        if let Some(parent) = config_path.parent() {
            fs::create_dir_all(parent)?;
        }
        let contents = serde_json::to_string_pretty(self)?;
        fs::write(config_path, contents)
    }

    fn get_config_path() -> PathBuf {
        dirs::config_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("nimble")
            .join("config.json")
    }
}
