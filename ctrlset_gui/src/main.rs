use eframe::{egui, App, Frame};
use egui::{CentralPanel, Context};
use std::process::Command;
use ctrlset_cli::Keybind;
use serde_json;
use egui_extras::{TableBuilder, Column};

struct MyApp {
    keybinds: Vec<Keybind>,
    error_message: Option<String>,
    new_name: String,
    new_key: String,
    new_app_name: String,
    search_pattern: String,
    fuzzy_search: bool,
}

impl Default for MyApp {
    fn default() -> Self {
        Self {
            keybinds: Vec::new(),
            error_message: None,
            new_name: "".to_string(),
            new_key: "".to_string(),
            new_app_name: "".to_string(),
            search_pattern: "".to_string(),
            fuzzy_search: false,
        }
    }
}

impl App for MyApp {
    fn update(&mut self, ctx: &Context, _frame: &mut Frame) {
        CentralPanel::default().show(ctx, |ui| {
            ui.heading("CtrlSet");

            ui.separator();
            ui.heading("Add New Keybind");
            egui::Grid::new("add_keybind_grid").num_columns(2).show(ui, |ui| {
                ui.label("Name:");
                ui.text_edit_singleline(&mut self.new_name);
                ui.end_row();

                ui.label("Key:");
                ui.text_edit_singleline(&mut self.new_key);
                ui.end_row();

                ui.label("App Name:");
                ui.text_edit_singleline(&mut self.new_app_name);
                ui.end_row();
            });

            if ui.button("Add Keybind").clicked() {
                let output = Command::new("target/debug/ctrlset_cli")
                    .args(&[
                        "add",
                        "-n",
                        &self.new_name,
                        "-k",
                        &self.new_key,
                        "-a",
                        &self.new_app_name,
                    ])
                    .output();
                
                match output {
                    Ok(output) => {
                        if !output.status.success() {
                            let stderr = String::from_utf8_lossy(&output.stderr);
                            self.error_message = Some(format!("CLI Error: {}", stderr));
                        } else {
                            self.new_name.clear();
                            self.new_key.clear();
                            self.new_app_name.clear();
                            self.refresh_keybinds();
                        }
                    }
                    Err(e) => self.error_message = Some(format!("Failed to execute process: {}", e)),
                }
            }

            ui.separator();
            
            if ui.button("Refresh Keybinds").clicked() {
                self.refresh_keybinds();
            }

            if let Some(error) = &self.error_message {
                ui.colored_label(egui::Color32::RED, error);
            }

            ui.separator();

            ui.heading("Search Keybinds");
            let search_changed = ui.text_edit_singleline(&mut self.search_pattern).changed();
            let fuzzy_changed = ui.checkbox(&mut self.fuzzy_search, "Fuzzy Search").changed();
            if search_changed || fuzzy_changed {
                self.search_keybinds();
            }

            ui.separator();

            TableBuilder::new(ui)
                .striped(true)
                .resizable(true)
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .column(Column::auto())
                .header(20.0, |mut header| {
                    header.col(|ui| {
                        ui.strong("App");
                    });
                    header.col(|ui| {
                        ui.strong("Name");
                    });
                    header.col(|ui| {
                        ui.strong("Key");
                    });
                    header.col(|ui| {
                        ui.strong("Actions");
                    });
                })
                .body(|mut body| {
                    for keybind in self.keybinds.clone() {
                        body.row(30.0, |mut row| {
                            row.col(|ui| {
                                ui.label(keybind.app_name());
                            });
                            row.col(|ui| {
                                ui.label(keybind.name());
                            });
                            row.col(|ui| {
                                ui.label(keybind.key());
                            });
                            row.col(|ui| {
                                if ui.button("Remove").clicked() {
                                    self.remove_keybind(keybind.name());
                                }
                            });
                        });
                    }
                });
        });
    }
}

impl MyApp {
    fn refresh_keybinds(&mut self) {
        let output = Command::new("target/debug/ctrlset_cli")
            .args(&["list", "--json"])
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    match serde_json::from_str(&stdout) {
                        Ok(keybinds) => {
                            self.keybinds = keybinds;
                            self.error_message = None;
                        },
                        Err(e) => self.error_message = Some(format!("Failed to parse keybinds: {}", e)),
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    self.error_message = Some(format!("CLI Error: {}", stderr));
                }
            }
            Err(e) => self.error_message = Some(format!("Failed to execute process: {}", e)),
        }
    }

    fn remove_keybind(&mut self, name: &str) {
        let output = Command::new("target/debug/ctrlset_cli")
            .args(&["remove", name])
            .output();

        match output {
            Ok(output) => {
                if !output.status.success() {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    self.error_message = Some(format!("CLI Error: {}", stderr));
                } else {
                    self.refresh_keybinds();
                }
            }
            Err(e) => self.error_message = Some(format!("Failed to execute process: {}", e)),
        }
    }

    fn search_keybinds(&mut self) {
        if self.search_pattern.is_empty() {
            self.refresh_keybinds();
            return;
        }

        let mut args = vec!["search".to_string(), self.search_pattern.clone(), "--json".to_string()];
        if self.fuzzy_search {
            args.push("--fuzzy".to_string());
        }

        let output = Command::new("target/debug/ctrlset_cli")
            .args(&args)
            .output();

        match output {
            Ok(output) => {
                if output.status.success() {
                    let stdout = String::from_utf8_lossy(&output.stdout);
                    match serde_json::from_str(&stdout) {
                        Ok(keybinds) => {
                            self.keybinds = keybinds;
                            self.error_message = None;
                        },
                        Err(e) => self.error_message = Some(format!("Failed to parse keybinds: {}", e)),
                    }
                } else {
                    let stderr = String::from_utf8_lossy(&output.stderr);
                    self.error_message = Some(format!("CLI Error: {}", stderr));
                }
            }
            Err(e) => self.error_message = Some(format!("Failed to execute process: {}", e)),
        }
    }
}


fn main() {
    let options = eframe::NativeOptions::default();
    let _ = eframe::run_native(
        "CtrlSet",
        options,
        Box::new(|_cc| Box::new(MyApp::default())),
    );
}
