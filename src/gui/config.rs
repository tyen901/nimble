use std::path::PathBuf;
use super::state::GuiConfig;

pub fn get_config_path() -> PathBuf {
    if let Some(config_dir) = dirs::config_dir() {
        config_dir.join("nimble").join("config.json")
    } else {
        PathBuf::from("nimble_config.json")
    }
}

pub fn ensure_config_dir() -> std::io::Result<()> {
    if let Some(config_dir) = dirs::config_dir() {
        let nimble_dir = config_dir.join("nimble");
        if !nimble_dir.exists() {
            std::fs::create_dir_all(nimble_dir)?;
        }
    }
    Ok(())
}

fn upgrade_config(mut config: GuiConfig) -> Result<GuiConfig, String> {
    while config.version() < GuiConfig::CURRENT_VERSION {
        config = match config.version() {
            1 => upgrade_v1_to_v2(config)?,
            2 => upgrade_v2_to_v3(config)?,
            // Add new version upgrades here
            v => return Err(format!("Unknown config version: {}", v)),
        };
    }
    Ok(config)
}

fn upgrade_v1_to_v2(mut config: GuiConfig) -> Result<GuiConfig, String> {
    // Example: Add new fields with defaults or transform existing ones
    config.set_version(2);
    Ok(config)
}

fn upgrade_v2_to_v3(mut config: GuiConfig) -> Result<GuiConfig, String> {
    // Future upgrade path
    config.set_version(3);
    Ok(config)
}

pub fn load_config() -> Result<GuiConfig, String> {
    let path = get_config_path();
    
    if !path.exists() {
        return Ok(GuiConfig::default());
    }

    let config_str = std::fs::read_to_string(&path)
        .map_err(|e| format!("Failed to read config file: {}", e))?;

    let config: GuiConfig = serde_json::from_str(&config_str)
        .map_err(|e| format!("Failed to parse config file: {}", e))?;

    // Try to upgrade if version is old
    if config.version() < GuiConfig::CURRENT_VERSION {
        let upgraded = upgrade_config(config)?;
        // Save the upgraded config
        save_config(&upgraded)?;
        Ok(upgraded)
    } else if config.version() > GuiConfig::CURRENT_VERSION {
        Err(format!(
            "Config version {} is newer than supported version {}",
            config.version(),
            GuiConfig::CURRENT_VERSION
        ))
    } else {
        Ok(config)
    }
}

pub fn save_config(config: &GuiConfig) -> Result<(), String> {
    ensure_config_dir()
        .map_err(|e| format!("Failed to create config directory: {}", e))?;

    let config_str = serde_json::to_string_pretty(config)
        .map_err(|e| format!("Failed to serialize config: {}", e))?;

    std::fs::write(get_config_path(), config_str)
        .map_err(|e| format!("Failed to write config file: {}", e))
}
