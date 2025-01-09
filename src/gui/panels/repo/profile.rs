use eframe::egui;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use crate::gui::widgets::PathPicker;
use crate::gui::state::{CommandMessage, GuiConfig};
use std::sync::mpsc::Sender;

#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Profile {
    pub name: String,
    pub repo_url: String,
    pub base_path: PathBuf,
}

impl Default for Profile {
    fn default() -> Self {
        Self {
            name: String::new(),
            repo_url: String::new(),
            base_path: PathBuf::new(),
        }
    }
}

pub struct ProfileManager {
    pub(crate) profiles: Vec<Profile>,
    pub(crate) selected_profile: Option<String>,
    editing_profile: Option<Profile>,
    pub(crate) path_picker: PathPicker,
}

impl Default for ProfileManager {
    fn default() -> Self {
        Self {
            profiles: Vec::new(),
            selected_profile: None,
            editing_profile: None,
            path_picker: PathPicker::new("Base Path:", "Select Mods Directory"),
        }
    }
}

impl ProfileManager {
    pub fn load_from_config(&mut self, config: &GuiConfig) {
        self.profiles = config.get_profiles().clone();
        self.selected_profile = config.get_selected_profile_name().clone();
        
        // Get the path before updating path_picker to avoid borrow issues
        let path = self.get_selected_profile()
            .map(|profile| profile.base_path.clone());
            
        if let Some(path) = path {
            self.path_picker.set_path(&path);
        }
    }

    pub fn save_to_config(&mut self, config: &mut GuiConfig) {
        config.set_profiles(self.profiles.clone());
        config.set_selected_profile(self.selected_profile.clone());
    }

    pub fn get_base_path(&self) -> PathBuf {
        self.path_picker.path()
    }

    pub fn get_selected_profile(&self) -> Option<&Profile> {
        self.selected_profile
            .as_ref()
            .and_then(|name| self.profiles.iter().find(|p| &p.name == name))
    }

    pub fn get_current_url(&self) -> Option<String> {
        self.get_selected_profile()
            .map(|profile| profile.repo_url.clone())
    }

    pub fn show_editor(&mut self, ui: &mut egui::Ui, sender: Option<&Sender<CommandMessage>>) -> bool {
        let mut changed = false;

        // Profile selector and management UI on a single line
        ui.horizontal(|ui| {
            ui.heading("Profile:");
            ui.add_space(4.0);
            
            // Profile dropdown with disconnect on change
            egui::ComboBox::new("profile_selector", "")
                .selected_text(self.selected_profile.as_deref().unwrap_or("Select Profile"))
                .show_ui(ui, |ui| {
                    for profile in &self.profiles {
                        let was_selected = self.selected_profile.as_ref().map(|s| s == &profile.name).unwrap_or(false);
                        if ui.selectable_value(
                            &mut self.selected_profile,
                            Some(profile.name.clone()),
                            &profile.name
                        ).clicked() && !was_selected {
                            // Profile changed, trigger disconnect
                            if let Some(sender) = sender {
                                sender.send(CommandMessage::Disconnect).ok();
                            }
                            changed = true;
                        }
                    }
                });

            ui.add_space(8.0);
            
            if ui.button("New").clicked() {
                self.editing_profile = Some(Default::default());
            }

            if self.selected_profile.is_some() {
                ui.add_space(4.0);
                if ui.button("Edit").clicked() {
                    let selected = self.selected_profile.as_ref().unwrap();
                    self.editing_profile = self.profiles
                        .iter()
                        .find(|p| &p.name == selected)
                        .cloned();
                }

                ui.add_space(4.0);
                if ui.button("Delete").clicked() {
                    let selected = self.selected_profile.as_ref().unwrap();
                    self.profiles.retain(|p| &p.name != selected);
                    self.selected_profile = None;
                    // Profile deleted, trigger disconnect
                    if let Some(sender) = sender {
                        sender.send(CommandMessage::Disconnect).ok();
                    }
                    changed = true;
                }
            }
        });

        // Show editor window if editing
        if let Some(editing) = &mut self.editing_profile {
            let mut editing_clone = editing.clone();
            egui::Window::new("Edit Profile")
                .show(ui.ctx(), |ui| {
                    ui.vertical(|ui| {
                        ui.horizontal(|ui| {
                            ui.label("Name:");
                            ui.text_edit_singleline(&mut editing_clone.name);
                        });
                        ui.horizontal(|ui| {
                            ui.label("Repository URL:");
                            ui.text_edit_singleline(&mut editing_clone.repo_url);
                        });
                        
                        // Improved path picker integration
                        ui.group(|ui| {
                            ui.label("Installation Path:");
                            ui.horizontal(|ui| {
                                ui.label(editing_clone.base_path.to_string_lossy().to_string());
                                if ui.button("📂 Browse").clicked() {
                                    if let Some(path) = rfd::FileDialog::new()
                                        .set_title("Select Installation Directory")
                                        .pick_folder() 
                                    {
                                        editing_clone.base_path = path;
                                    }
                                }
                            });
                        });

                        ui.add_space(8.0);
                        ui.horizontal(|ui| {
                            if ui.button("Save").clicked() {
                                if !editing_clone.name.is_empty() {
                                    self.profiles.retain(|p| p.name != editing_clone.name);
                                    self.profiles.push(editing_clone.clone());
                                    self.selected_profile = Some(editing_clone.name.clone());
                                    self.path_picker.set_path(&editing_clone.base_path);
                                    if let Some(sender) = sender {
                                        sender.send(CommandMessage::ConfigChanged).ok();
                                    }
                                    self.editing_profile = None; // Clear editing state
                                    changed = true;
                                }
                            }
                            if ui.button("Cancel").clicked() {
                                self.editing_profile = None; // Clear editing state
                                changed = true;
                            }
                        });
                    });
                });
        }

        changed
    }

    pub fn set_selected(&mut self, profile: Option<String>) {
        self.selected_profile = profile;
    }

    pub fn set_editing(&mut self, profile: Option<Profile>) {
        self.editing_profile = profile;
    }

    pub fn get_editing(&self) -> Option<&Profile> {
        self.editing_profile.as_ref()
    }

    pub fn update_profiles<F>(&mut self, f: F) 
    where F: FnOnce(&mut Vec<Profile>) {
        f(&mut self.profiles);
    }

    // Method to modify editing_profile since it's private
    pub fn set_editor_profile(&mut self, profile: Option<Profile>) {
        self.editing_profile = profile;
    }
}
