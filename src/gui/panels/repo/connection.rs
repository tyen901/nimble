use eframe::egui;
use std::sync::mpsc::Sender;
use url::Url;
use crate::gui::state::{CommandMessage, GuiState};
use crate::gui::panels::repo::state::ConnectionState;
use crate::repository::Repository;
use super::state::RepoPanelState;
use crate::mod_cache::ModCache;

fn ensure_valid_url(url: &str) -> Result<String, String> {
let url = if !url.starts_with("http://") && !url.starts_with("https://") {
        format!("https://{}", url)
    } else {
        url.to_string()
    };

    // Try to parse the URL and ensure it ends with repo.json if not already
    let mut parsed_url = Url::parse(&url)
        .map_err(|e| format!("Invalid URL: {}", e))?;
    
    if !parsed_url.path().ends_with("repo.json") {
        let new_path = if parsed_url.path().ends_with('/') {
            format!("{}repo.json", parsed_url.path())
        } else {
            format!("{}/repo.json", parsed_url.path())
        };
        parsed_url.set_path(&new_path);
    }

    Ok(parsed_url.to_string())
}

fn analyze_json_error(json_str: &str, error: serde_json::Error) -> String {
    // Get the error message string first
    let error_str = error.to_string();
    
    // Get the field name from the error message
    let field_name = if let Some(field) = error_str.split_once("missing field `") {
        if let Some(field_name) = field.1.split('`').next() {
            field_name
        } else {
            "unknown"
        }
    } else {
        "unknown"
    };

    // Try to parse as Value to analyze the actual JSON structure
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(json_str) {
        if let Some(obj) = value.as_object() {
            let mut found_fields = Vec::new();
            
            // Search through all fields recursively
            fn find_fields(obj: &serde_json::Value, name: &str, found: &mut Vec<String>, path: &str) {
                match obj {
                    serde_json::Value::Object(map) => {
                        for (key, value) in map {
                            let new_path = if path.is_empty() {
                                key.clone()
                            } else {
                                format!("{}.{}", path, key)
                            };
                            
                            if key.to_lowercase() == name.to_lowercase() {
                                found.push(format!("Found field '{}' at path '{}'", key, path));
                            }
                            find_fields(value, name, found, &new_path);
                        }
                    },
                    serde_json::Value::Array(arr) => {
                        for (idx, value) in arr.iter().enumerate() {
                            let new_path = format!("{}[{}]", path, idx);
                            find_fields(value, name, found, &new_path);
                        }
                    },
                    _ => {}
                }
            }
            
            find_fields(&value, field_name, &mut found_fields, "");
            
            if !found_fields.is_empty() {
                return format!(
                    "Failed to parse repository data: {}\n{}\nExpected field name: '{}'\nFound similar fields:\n{}",
                    error,
                    "-".repeat(40),
                    field_name,
                    found_fields.join("\n")
                );
            }
        }
    }

    format!("Failed to parse repository data: {}", error_str)
}

pub fn connect_to_server(state: &mut RepoPanelState, repo_url: &str, sender: &Sender<CommandMessage>) {
    let profile = match state.profile_manager().get_selected_profile().cloned() {
        Some(p) => p,
        None => {
            eprintln!("Connection failed: No profile selected");
            sender.send(CommandMessage::ConnectionError("No profile selected".to_string())).ok();
            return;
        }
    };

    // Load existing cache but don't update it
    if let Ok(cache) = ModCache::from_disk_or_empty(&profile.base_path) {
        state.load_cache(&cache);
    }

    // Validate URL and ensure it points to repo.json
    let repo_url = match ensure_valid_url(repo_url) {
        Ok(url) => url,
        Err(e) => {
            eprintln!("Connection failed: {}", e);
            sender.send(CommandMessage::ConnectionError(e)).ok();
            return;
        }
    };

    println!("Attempting to connect to repository: {}", repo_url);
    
    // Start connection process
    state.set_connecting();
    let sender = sender.clone();
    
    std::thread::spawn(move || {
        let mut agent = ureq::agent();
        match Repository::new(&repo_url, &mut agent) {
            Ok(repo) => {
                println!("Successfully connected to repository {}", repo_url);
                sender.send(CommandMessage::ConnectionComplete(repo))
            },
            Err(e) => {
                eprintln!("Failed to connect to repository {}: {}", repo_url, e);
                sender.send(CommandMessage::ConnectionError(format!(
                    "Failed to connect to {}: {}", 
                    repo_url, 
                    e.to_string()
                )))
            },
        }.ok();
    });
}

pub fn disconnect(state: &mut RepoPanelState, sender: &Sender<CommandMessage>) {
    state.disconnect();
    sender.send(CommandMessage::Disconnect).ok();
}