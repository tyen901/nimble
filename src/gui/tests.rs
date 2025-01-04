#[cfg(test)]
mod tests {
    use super::super::state::{GuiState, GuiConfig, CommandMessage, CommandChannels};
    use super::super::panels::{sync_panel::SyncPanel, launch_panel::LaunchPanel, gen_srf_panel::GenSrfPanel};
    use std::path::PathBuf;
    use tempfile::TempDir;
    use std::env;

    const PEANUT_REPO_URL: &str = "http://swifty.peanutcommunityarma.com/";

    #[test]
    fn test_gui_state_defaults() {
        let state = GuiState::Idle;
        assert!(matches!(state, GuiState::Idle));
    }

    #[test]
    fn test_sync_panel() {
        let panel = SyncPanel::default();
        assert!(panel.repo_url.is_empty());
        assert!(panel.base_path.is_empty());
    }

    #[test]
    fn test_launch_panel() {
        let panel = LaunchPanel::default();
        assert!(panel.base_path.is_empty());
    }

    #[test]
    fn test_gen_srf_panel() {
        let panel = GenSrfPanel::default();
        assert!(panel.base_path.is_empty());
    }

    #[test]
    fn test_command_channels() {
        let channels = CommandChannels::new();
        channels.sender.send(CommandMessage::SyncComplete).unwrap();
        assert!(matches!(channels.receiver.try_recv(), Ok(CommandMessage::SyncComplete)));
    }

    #[test]
    fn test_sync_progress_message() {
        let channels = CommandChannels::new();
        channels.sender.send(CommandMessage::SyncProgress {
            file: "test.pbo".to_string(),
            progress: 0.5,
            processed: 1,
            total: 2
        }).unwrap();
        
        match channels.receiver.try_recv().unwrap() {
            CommandMessage::SyncProgress { file, progress, processed, total } => {
                assert_eq!(file, "test.pbo");
                assert_eq!(progress, 0.5);
                assert_eq!(processed, 1);
                assert_eq!(total, 2);
            },
            _ => panic!("Wrong message type received"),
        }
    }

    #[test]
    fn test_gen_srf_progress_message() {
        let channels = CommandChannels::new();
        channels.sender.send(CommandMessage::GenSrfProgress {
            current_mod: "@test_mod".to_string(),
            progress: 0.5,
            processed: 1,
            total: 2
        }).unwrap();
        
        match channels.receiver.try_recv().unwrap() {
            CommandMessage::GenSrfProgress { current_mod, progress, processed, total } => {
                assert_eq!(current_mod, "@test_mod");
                assert_eq!(progress, 0.5);
                assert_eq!(processed, 1);
                assert_eq!(total, 2);
            },
            _ => panic!("Wrong message type received"),
        }
    }

    #[test]
    fn test_sync_panel_state_handling() {
        let mut panel = SyncPanel::default();
        let state = GuiState::Syncing {
            progress: 0.5,
            current_file: "test.pbo".to_string(),
            files_processed: 1,
            total_files: 2,
        };
        
        // Create a test UI context and verify panel responds to state
        let ctx = egui::Context::default();
        ctx.run(|ctx| {
            egui::Window::new("test").show(ctx, |ui| {
                panel.show(ui, &state, None);
                // UI assertions could be added here if needed
            });
        });
    }

    #[test]
    fn test_gen_srf_panel_state_handling() {
        let mut panel = GenSrfPanel::default();
        let state = GuiState::GeneratingSRF {
            progress: 0.5,
            current_mod: "@test_mod".to_string(),
            mods_processed: 1,
            total_mods: 2,
        };
        
        // Create a test UI context and verify panel responds to state
        let ctx = egui::Context::default();
        ctx.run(|ctx| {
            egui::Window::new("test").show(ctx, |ui| {
                panel.show(ui, &state, None);
                // UI assertions could be added here if needed
            });
        });
    }

    #[test]
    fn test_gui_config_serialization() {
        use std::path::PathBuf;
        
        let config = GuiConfig {
            repo_url: "https://test.com".to_string(),
            base_path: PathBuf::from("/test/path"),
            window_size: (100.0, 100.0),
        };
        
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: GuiConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.repo_url, deserialized.repo_url);
        assert_eq!(config.base_path, deserialized.base_path);
        assert_eq!(config.window_size, deserialized.window_size);
    }

    #[test]
    fn test_sync_with_peanut_repo() {
        let temp_dir = TempDir::new().unwrap();
        let mut panel = SyncPanel::default();
        panel.repo_url = PEANUT_REPO_URL.to_string();
        panel.base_path = temp_dir.path().to_string_lossy().to_string();

        let channels = CommandChannels::new();
        panel.start_sync_dry_run(channels.sender.clone());

        // Check for repository response
        let mut got_response = false;
        while let Ok(msg) = channels.receiver.try_recv() {
            match msg {
                CommandMessage::SyncProgress { .. } | CommandMessage::SyncComplete => {
                    got_response = true;
                    break;
                }
                CommandMessage::SyncError(e) => panic!("Failed to contact Peanut repo: {}", e),
                _ => continue,
            }
        }
        assert!(got_response, "Should have received response from Peanut repository");
    }

    #[test]
    fn test_gen_srf_with_ace_test_mod() {
        let project_root = env::var("CARGO_MANIFEST_DIR").unwrap();
        let ace_test_path = PathBuf::from(project_root).join("test_files").join("@ace");
        assert!(ace_test_path.exists(), "Test mod @ace not found in test_files");
        
        let mut panel = GenSrfPanel::default();
        panel.base_path = ace_test_path.parent().unwrap().to_string_lossy().to_string();

        let channels = CommandChannels::new();
        panel.start_gen_srf(channels.sender.clone());

        // Verify ACE mod processing
        let mut processed_ace = false;
        while let Ok(msg) = channels.receiver.try_recv() {
            match msg {
                CommandMessage::GenSrfProgress { current_mod, .. } => {
                    if current_mod == "@ace" {
                        processed_ace = true;
                        break;
                    }
                }
                CommandMessage::GenSrfComplete => break;
                CommandMessage::GenSrfError(e) => panic!("Failed to process @ace test mod: {}", e),
                _ => continue,
            }
        }
        assert!(processed_ace, "Should have processed @ace test mod");
    }

    #[test]
    fn test_gui_config_with_real_paths() {
        let config = GuiConfig {
            repo_url: PEANUT_REPO_URL.to_string(),
            base_path: PathBuf::from(env::var("CARGO_MANIFEST_DIR").unwrap()).join("test_files"),
            window_size: (800.0, 600.0),
        };
        
        let serialized = serde_json::to_string(&config).unwrap();
        let deserialized: GuiConfig = serde_json::from_str(&serialized).unwrap();
        
        assert_eq!(config.repo_url, PEANUT_REPO_URL);
        assert_eq!(config.base_path.file_name().unwrap(), "test_files");
    }
}
