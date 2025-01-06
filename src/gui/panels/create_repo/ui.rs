use eframe::egui;
use crate::repository::{Repository, Server, Mod};
use super::state::CreateRepoPanelState;

pub fn render_panel(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    let panel_width = ui.available_width().min(1000.0);
    ui.set_min_width(panel_width);

    egui::ScrollArea::vertical()
        .auto_shrink([false, false])
        .show(ui, |ui| {
            ui.vertical(|ui| {
                state.status.show(ui);
                ui.add_space(16.0);
                render_repository_setup(ui, state);
                
                if state.last_scanned_path.is_some() {
                    ui.add_space(16.0);
                    
                    ui.horizontal(|ui| {
                        // Left column
                        ui.vertical(|ui| {
                            ui.set_min_width(400.0);
                            render_basic_settings(ui, state);
                            ui.add_space(16.0);
                            render_servers_config(ui, &mut state.repo);
                            ui.add_space(16.0);
                            render_options(ui, state);
                        });

                        ui.add_space(16.0);

                        // Right column - mods list
                        ui.vertical(|ui| {
                            ui.set_min_width(300.0);
                            render_mods_section(ui, state);
                        });
                    });

                    ui.add_space(16.0);
                    render_save_button(ui, state);
                }
            });
        });
}

fn render_repository_setup(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    ui.heading("Repository Setup");
    state.base_path.show(ui);
    
    // Show a hint if no path is selected yet
    if state.last_scanned_path.is_none() {
        ui.label("Select a folder to begin creating or editing a repository");
    }
}

fn render_main_settings(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    ui.vertical(|ui| {
        render_basic_settings(ui, state);
        ui.add_space(8.0);
        render_mods_section(ui, state);
    });
}

fn render_basic_settings(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    ui.group(|ui| {
        ui.set_min_width(400.0);
        ui.heading("Basic Settings");
        ui.horizontal(|ui| {
            ui.label("Repository Name:");
            ui.text_edit_singleline(&mut state.repo.repo_name);
        });
        ui.horizontal(|ui| {
            ui.label("Client Parameters:");
            ui.text_edit_singleline(&mut state.repo.client_parameters);
        });
        ui.horizontal(|ui| {
            ui.label("Version:");
            ui.label(&state.repo.version);  // Changed to display-only label
        });
    });
}

fn render_options(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    ui.group(|ui| {
        ui.set_min_width(400.0);
        ui.heading("Cleanup Options");
        ui.add_space(8.0);

        ui.checkbox(&mut state.clean_options.force_lowercase, "Force lowercase filenames when saving");
        ui.add_space(8.0);
        ui.checkbox(&mut state.clean_options.cleanup_files, "Remove excluded files when saving");
        
        if state.clean_options.cleanup_files {
            ui.add_space(8.0);
            ui.group(|ui| {
                ui.label("Excluded Files & Directories (separated by ;):");
                ui.text_edit_multiline(&mut state.clean_options.excluded_files);
            });
        }
    });
}

fn render_mods_section(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    ui.group(|ui| {
        ui.set_min_width(300.0);
        ui.heading("Required Mods");
        ui.add_space(8.0);
        
        if state.show_update_prompt {
            render_update_prompt(ui, state);
            ui.separator();
        }

        egui::ScrollArea::vertical()
            .max_height(500.0) // Increased height since we have more vertical space
            .id_source("mods_list")
            .show(ui, |ui| {
                render_mods_list(ui, &state.repo.required_mods);
            });
    });
}

fn render_mods_list(ui: &mut egui::Ui, mods: &[Mod]) {
    if !mods.is_empty() {
        ui.vertical(|ui| {
            for mod_entry in mods {
                ui.add(egui::Label::new(&mod_entry.mod_name));
            }
        });
    } else {
        ui.label("No mods found");
    }
}

fn render_update_prompt(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    ui.horizontal(|ui| {
        ui.label("Changes detected!");
        if ui.button("Update List").clicked() {
            if let Some(new_mods) = state.pending_mods.take() {
                state.repo.required_mods = new_mods;
                state.status.set_info("Updated mod list");
            }
            state.show_update_prompt = false;
        }
        if ui.button("Keep Existing").clicked() {
            state.pending_mods = None;
            state.show_update_prompt = false;
            state.status.set_info("Kept existing mod list");
        }
    });
}

fn render_servers_config(ui: &mut egui::Ui, repo: &mut Repository) {
    ui.group(|ui| {
        ui.heading("Servers");
        if repo.servers.is_empty() {
            if ui.button("Add Server").clicked() {
                repo.servers.push(Server {
                    name: String::new(),
                    address: "127.0.0.1".parse().unwrap(),
                    port: 2302,
                    password: String::new(),
                    battle_eye: true,
                });
            }
        } else {
            render_server_entry(ui, &mut repo.servers[0]);
        }
    });
}

fn render_server_entry(ui: &mut egui::Ui, server: &mut Server) {
    ui.vertical(|ui| {
        ui.horizontal(|ui| {
            ui.label("Name:");
            ui.text_edit_singleline(&mut server.name);
        });
        ui.horizontal(|ui| {
            ui.label("Address:");
            let mut addr_str = server.address.to_string();
            if ui.text_edit_singleline(&mut addr_str).changed() {
                if let Ok(new_addr) = addr_str.parse() {
                    server.address = new_addr;
                }
            }
        });
        ui.horizontal(|ui| {
            ui.label("Port:");
            ui.add(egui::DragValue::new(&mut server.port)
                .clamp_range(1024..=65535)
                .speed(1)
            );
        });
        ui.horizontal(|ui| {
            ui.label("Password:");
            ui.text_edit_singleline(&mut server.password);
        });
        ui.checkbox(&mut server.battle_eye, "BattlEye");
    });
}

fn render_save_button(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    ui.add_space(8.0);
    
    let button = egui::Button::new("Save Repository")
        .fill(egui::Color32::from_rgb(100, 200, 100));
    
    if ui.add_sized(ui.available_size_before_wrap(), button).clicked() {
        let path = state.base_path.path();
        if path.exists() {
            // Only clean files if the cleanup option is enabled
            if state.clean_options.cleanup_files {
                if let Err(e) = super::actions::clean_directory(
                    &path,
                    state.clean_options.force_lowercase,
                    &state.clean_options.excluded_files,
                ) {
                    state.status.set_error(format!("Cleanup failed: {}", e));
                    return;
                }
            } else if state.clean_options.force_lowercase {
                // If only lowercase is enabled, just do that
                if let Err(e) = super::actions::rename_to_lowercase(&path) {
                    state.status.set_error(format!("Lowercase conversion failed: {}", e));
                    return;
                }
            }
            
            match super::actions::save_repository(&path, &mut state.repo) {
                Ok(_) => state.status.set_info("Saved repository successfully"),
                Err(e) => state.status.set_error(format!("Failed to save: {}", e)),
            }
        }
    }
}
