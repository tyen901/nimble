use eframe::egui;
use crate::repository::{Repository, Server, Mod};
use super::state::CreateRepoPanelState;

pub fn render_panel(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    state.status.show(ui);
    ui.add_space(8.0);

    render_repository_setup(ui, state);
    
    // Only show the rest of the UI if a valid path is selected and scanned
    if state.last_scanned_path.is_some() {
        ui.add_space(8.0);
        render_main_settings(ui, state);
        render_servers_config(ui, &mut state.repo);
        render_save_button(ui, state);
    }
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
    ui.horizontal(|ui| {
        ui.vertical(|ui| {
            render_basic_settings(ui, state);
            render_options(ui, state);
        });
        ui.vertical(|ui| {
            render_mods_section(ui, state);
        });
    });
}

fn render_basic_settings(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    ui.group(|ui| {
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
            ui.text_edit_singleline(&mut state.repo.version);
        });
    });
}

fn render_options(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    ui.group(|ui| {
        ui.heading("Options");
        ui.checkbox(&mut state.auto_increment_version, "Auto-increment version");
    });
}

fn render_mods_section(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    ui.group(|ui| {
        ui.heading("Required Mods");
        
        if state.show_update_prompt {
            render_update_prompt(ui, state);
            ui.separator();
        }

        egui::ScrollArea::vertical()
            .max_height(200.0)
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
    if !mods.is_empty() {
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
    ui.horizontal(|ui| {
        ui.label("Name:");
        ui.text_edit_singleline(&mut server.name);
        ui.label("Address:");
        if ui.button(server.address.to_string()).clicked() {
            if let Ok(new_addr) = "127.0.0.1".parse() {
                server.address = new_addr;
            }
        }
        ui.label("Port:");
        ui.add(egui::DragValue::new(&mut server.port).speed(1));
        ui.label("Password:");
        ui.text_edit_singleline(&mut server.password);
        ui.checkbox(&mut server.battle_eye, "BattlEye");
    });
}

fn render_save_button(ui: &mut egui::Ui, state: &mut CreateRepoPanelState) {
    ui.separator();
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
