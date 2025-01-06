use eframe::egui;
use crate::repository::{Repository, Server, Mod};
use super::state::CreateRepoPanelState;

pub fn render_panel(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    egui::ScrollArea::vertical()
        .show(ui, |ui| {
            state.status.show(ui);
            ui.add_space(8.0);

            render_repository_setup(ui, state);
            
            if state.last_scanned_path.is_some() {
                ui.add_space(8.0);
                render_main_settings(ui, state);
                render_servers_config(ui, &mut state.repo);
                ui.separator();
                render_options(ui, state);
                render_save_button(ui, state);
            }
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
    ui.collapsing("Options", |ui| {
        ui.collapsing("Cleaning Options", |ui| {
            ui.checkbox(&mut state.clean_options.auto_clean, "Auto-clean directory");
            
            if state.clean_options.auto_clean {
                ui.indent("clean_options", |ui| {
                    ui.checkbox(&mut state.clean_options.force_lowercase, "Force lowercase filenames");
                    
                    // File filter section
                    ui.group(|ui| {
                        ui.label("File Filters:");
                        ui.horizontal(|ui| {
                            let text_edit = ui.text_edit_singleline(&mut state.clean_options.new_filter);
                            let add_filter = !state.clean_options.new_filter.is_empty() && 
                                (text_edit.lost_focus() && ui.input(|i| i.key_pressed(egui::Key::Enter)) || 
                                ui.button("+").clicked());
                            
                            if add_filter {
                                state.clean_options.file_filters.push(state.clean_options.new_filter.clone());
                                state.clean_options.new_filter.clear();
                            }
                        });
                        
                        ui.separator();
                        
                        egui::ScrollArea::vertical()
                            .max_height(100.0)
                            .show(ui, |ui| {
                                for filter_idx in (0..state.clean_options.file_filters.len()).collect::<Vec<_>>() {
                                    ui.horizontal(|ui| {
                                        ui.label(&state.clean_options.file_filters[filter_idx]);
                                        if ui.small_button("✖").clicked() {
                                            state.clean_options.file_filters.remove(filter_idx);
                                        }
                                    });
                                }
                            });
                    });
                });
            }
        });
    });
}

fn render_mods_section(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    ui.collapsing("Required Mods", |ui| {
        ui.set_min_width(400.0);
        
        if state.show_update_prompt {
            render_update_prompt(ui, state);
            ui.separator();
        }

        egui::ScrollArea::vertical()
            .max_height(300.0)
            .show(ui, |ui| {
                render_mods_list(ui, &state.repo.required_mods);
            });
    });
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

fn render_mods_list(ui: &mut egui::Ui, mods: &[Mod]) {
    if (!mods.is_empty()) {
        for mod_entry in mods {
            ui.horizontal(|ui| {
                ui.label(&mod_entry.mod_name);
                ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                    ui.label(if mod_entry.enabled { "✓" } else { "✗" });
                });
            });
        }
    } else {
        ui.label("No mods found");
    }
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
    ui.horizontal(|ui| {
        if ui.button("Save Repository").clicked() {
            let path = state.base_path.path();
            if path.exists() {
                match super::actions::save_repository(&path, &mut state.repo) {
                    Ok(_) => state.status.set_info("Saved repository successfully"),
                    Err(e) => state.status.set_error(format!("Failed to save: {}", e)),
                }
            }
        }
    });
}
