use eframe::egui;
use egui::{vec2, Context, Key, KeyboardShortcut, Modifiers, RichText, Sense, Ui};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq)]
struct Keybind {
    keys: String,
    description: String,
    application: String,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum Mode {
    Normal,
    Insert,
    Search,
    Command,
    AppFilter,
}

struct AppState {
    keybinds: Vec<Keybind>,
    filtered_indices: Vec<usize>,
    selected_cell: (usize, usize),
    mode: Mode,
    search_query: String,
    command_buffer: String,
    status_message: String,
    application_filter: Option<String>,
    app_search_query: String,
    temp_edit_buffer: String,
    is_listening_for_keybind: bool,
    should_quit: bool,
    last_state: Option<Vec<Keybind>>,
    // Flag to prevent the key that triggers listening mode from being captured.
    ignore_next_input_frame: bool,
}

impl Default for AppState {
    fn default() -> Self {
        let mut app = Self {
            keybinds: vec![],
            filtered_indices: vec![],
            selected_cell: (0, 0),
            mode: Mode::Normal,
            search_query: String::new(),
            command_buffer: String::new(),
            status_message: "Welcome to ctrlset!".to_string(),
            application_filter: None,
            app_search_query: String::new(),
            temp_edit_buffer: String::new(),
            is_listening_for_keybind: false,
            should_quit: false,
            last_state: None,
            ignore_next_input_frame: false,
        };
        app.load_keybinds();
        app.refilter();
        app
    }
}

fn get_data_path() -> PathBuf {
    PathBuf::from("keybinds.json")
}

impl AppState {
    fn save_keybinds(&mut self) {
        let path = get_data_path();
        match serde_json::to_string_pretty(&self.keybinds) {
            Ok(json) => {
                if fs::write(path, json).is_ok() {
                    self.status_message = "Saved successfully!".to_string();
                } else {
                    self.status_message = "Error: Failed to write to file.".to_string();
                }
            }
            Err(_) => {
                self.status_message = "Error: Failed to serialize keybinds.".to_string();
            }
        }
    }

    fn load_keybinds(&mut self) {
        let path = get_data_path();
        if path.exists() {
            if let Ok(data) = fs::read_to_string(path) {
                if let Ok(keybinds) = serde_json::from_str(&data) {
                    self.keybinds = keybinds;
                    self.status_message = "Keybinds loaded.".to_string();
                } else {
                    self.status_message = "Error: Could not parse keybinds.json.".to_string();
                }
            }
        } else {
            self.keybinds = vec![
                Keybind {
                    keys: "Ctrl+S".into(),
                    description: "Save file".into(),
                    application: "VS Code".into(),
                },
                Keybind {
                    keys: "J".into(),
                    description: "Move down".into(),
                    application: "Vim".into(),
                },
                Keybind {
                    keys: "K".into(),
                    description: "Move up".into(),
                    application: "Vim".into(),
                },
            ];
            self.status_message = "Loaded example keybinds.".to_string();
        }
    }

    fn refilter(&mut self) {
        let matcher = SkimMatcherV2::default();
        let app_filter = self.application_filter.as_deref();
        let search_query = &self.search_query.to_lowercase();

        self.filtered_indices = self
            .keybinds
            .iter()
            .enumerate()
            .filter_map(|(idx, kb)| {
                if let Some(filter) = app_filter {
                    if kb.application != filter {
                        return None;
                    }
                }

                if search_query.is_empty() {
                    Some(idx)
                } else {
                    let combined_string =
                        format!("{} {} {}", kb.keys, kb.description, kb.application);
                    if matcher
                        .fuzzy_match(&combined_string, search_query)
                        .is_some()
                    {
                        Some(idx)
                    } else {
                        None
                    }
                }
            })
            .collect();
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        let (row, col) = self.selected_cell;
        let num_rows = self.filtered_indices.len();
        if num_rows == 0 {
            self.selected_cell = (0, 0);
        } else {
            self.selected_cell.0 = row.min(num_rows.saturating_sub(1));
        }
        self.selected_cell.1 = col.min(2);
    }

    fn enter_insert_mode(&mut self) {
        if self.filtered_indices.is_empty() {
            return;
        }
        self.mode = Mode::Insert;
        let (row_idx, col_idx) = self.selected_cell;
        let real_idx = self.filtered_indices[row_idx];

        self.temp_edit_buffer = match col_idx {
            0 => {
                self.is_listening_for_keybind = true;
                // Set the flag to ignore the input on the very next frame.
                self.ignore_next_input_frame = true;
                self.keybinds[real_idx].keys.clone()
            }
            1 => self.keybinds[real_idx].description.clone(),
            2 => self.keybinds[real_idx].application.clone(),
            _ => String::new(),
        };
    }

    fn exit_insert_mode(&mut self, saved: bool) {
        self.is_listening_for_keybind = false;

        if saved {
            self.save_last_state();
            let (row_idx, col_idx) = self.selected_cell;
            if let Some(real_idx) = self.filtered_indices.get(row_idx) {
                let kb = &mut self.keybinds[*real_idx];
                match col_idx {
                    0 => kb.keys = self.temp_edit_buffer.clone(),
                    1 => kb.description = self.temp_edit_buffer.clone(),
                    2 => kb.application = self.temp_edit_buffer.clone(),
                    _ => {}
                }
            }
        }
        self.mode = Mode::Normal;
        self.temp_edit_buffer.clear();
    }

    fn save_last_state(&mut self) {
        self.last_state = Some(self.keybinds.clone());
    }

    fn undo(&mut self) {
        if let Some(last_state) = self.last_state.take() {
            self.keybinds = last_state;
            self.refilter();
            self.status_message = "Undo successful.".to_string();
        } else {
            self.status_message = "Nothing to undo.".to_string();
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native("ctrlset", options, Box::new(|_cc| Box::new(App::new())))
}

struct App {
    state: AppState,
}

impl App {
    fn new() -> Self {
        Self {
            state: AppState::default(),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let state = &mut self.state;

        if state.is_listening_for_keybind {
            handle_key_capture(ctx, state);
        } else {
            handle_global_input(ctx, state);
        }

        if state.should_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            draw_main_table(ui, state);
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            draw_status_bar(ui, state);
        });

        if state.mode == Mode::AppFilter {
            draw_app_filter_popup(ctx, state);
        }
    }
}

fn handle_key_capture(ctx: &Context, state: &mut AppState) {
    // If the ignore flag is set, unset it and skip this frame.
    if state.ignore_next_input_frame {
        state.ignore_next_input_frame = false;
        return;
    }

    ctx.input(|i| {
        if i.key_pressed(Key::Escape) {
            state.exit_insert_mode(false);
            return;
        }

        // Find the main key that was just pressed.
        let pressed_key = i.events.iter().find_map(|e| {
            if let egui::Event::Key {
                key, pressed: true, ..
            } = e
            {
                Some(*key)
            } else {
                None
            }
        });

        if let Some(key) = pressed_key {
            let mut parts = Vec::new();
            let mods = i.modifiers;

            // Only capture if at least one non-modifier key is pressed.
            if !is_key_just_a_modifier(key) {
                if mods.ctrl {
                    parts.push("Ctrl".to_string());
                }
                if mods.alt {
                    parts.push("Alt".to_string());
                }
                if mods.shift {
                    parts.push("Shift".to_string());
                }
                if mods.mac_cmd {
                    parts.push("Cmd".to_string());
                }

                parts.push(format!("{:?}", key));
                state.temp_edit_buffer = parts.join("+");
                state.exit_insert_mode(true);
            }
        }
    });
}

// This function now correctly checks if a `Key` is one of the few that
// egui reports as a key event but which we consider a modifier-only action.
// The main modifiers (Ctrl, Shift, Alt) are handled by `i.modifiers` and don't generate these events.
fn is_key_just_a_modifier(key: Key) -> bool {
    matches!(
        key,
        Key::F1
            | Key::F2
            | Key::F3
            | Key::F4
            | Key::F5
            | Key::F6
            | Key::F7
            | Key::F8
            | Key::F9
            | Key::F10
            | Key::F11
            | Key::F12
            | Key::F13
            | Key::F14
            | Key::F15
            | Key::F16
            | Key::F17
            | Key::F18
            | Key::F19
            | Key::F20 // We no longer need to check for other keys like Tab, Enter, etc., here.
                       // We want to be able to bind them. The calling logic will handle them.
    )
}

fn handle_global_input(ctx: &Context, state: &mut AppState) {
    match state.mode {
        Mode::Normal => handle_normal_mode_input(ctx, state),
        Mode::Insert => handle_insert_mode_input(ctx, state),
        Mode::Search => handle_search_mode_input(ctx, state),
        Mode::Command => handle_command_mode_input(ctx, state),
        Mode::AppFilter => handle_app_filter_input(ctx, state),
    }
}

fn handle_normal_mode_input(ctx: &Context, state: &mut AppState) {
    let mut repeated_g = false;
    let mut repeated_d = false;
    ctx.input(|i| {
        if i.key_pressed(Key::G) && i.key_down(Key::G) {
            repeated_g = true;
        }
        if i.key_pressed(Key::D) && i.key_down(Key::D) {
            repeated_d = true;
        }
    });

    if repeated_g {
        state.selected_cell.0 = 0;
    }
    if repeated_d && !state.filtered_indices.is_empty() {
        state.save_last_state();
        let real_idx_to_remove = state.filtered_indices[state.selected_cell.0];
        state.keybinds.remove(real_idx_to_remove);
        state.refilter();
        state.clamp_selection();
        state.status_message = "Keybind deleted.".to_string();
    }

    ctx.input_mut(|i| {
        if i.consume_key(Modifiers::NONE, Key::J) {
            let num_rows = state.filtered_indices.len();
            if num_rows > 0 {
                state.selected_cell.0 = (state.selected_cell.0 + 1).min(num_rows - 1);
            }
        }
        if i.consume_key(Modifiers::NONE, Key::K) {
            state.selected_cell.0 = state.selected_cell.0.saturating_sub(1);
        }
        if i.consume_key(Modifiers::NONE, Key::L) {
            state.selected_cell.1 = (state.selected_cell.1 + 1).min(2);
        }
        if i.consume_key(Modifiers::NONE, Key::H) {
            state.selected_cell.1 = state.selected_cell.1.saturating_sub(1);
        }
        if i.consume_key(Modifiers::SHIFT, Key::G) {
            state.selected_cell.0 = state.filtered_indices.len().saturating_sub(1);
        }

        if i.consume_key(Modifiers::NONE, Key::I) {
            state.enter_insert_mode();
        }
        if i.consume_key(Modifiers::NONE, Key::Slash) {
            state.mode = Mode::Search;
            state.search_query.clear();
        }
        if i.consume_key(Modifiers::SHIFT, Key::Semicolon) {
            state.mode = Mode::Command;
            state.command_buffer.clear();
        }

        if i.consume_key(Modifiers::NONE, Key::O) || i.consume_key(Modifiers::SHIFT, Key::O) {
            let is_shift = i.modifiers.shift;
            state.save_last_state();
            let new_kb = Keybind {
                keys: "".into(),
                description: "".into(),
                application: state.application_filter.clone().unwrap_or_default(),
            };
            if is_shift {
                let insert_pos = state
                    .filtered_indices
                    .get(state.selected_cell.0)
                    .cloned()
                    .unwrap_or(0);
                state.keybinds.insert(insert_pos, new_kb);
            } else {
                let insert_pos = state
                    .filtered_indices
                    .get(state.selected_cell.0)
                    .map(|&idx| idx + 1)
                    .unwrap_or(state.keybinds.len());
                state.keybinds.insert(insert_pos, new_kb);
                state.selected_cell.0 = state.selected_cell.0.saturating_add(1);
            }
            state.refilter();
            state.enter_insert_mode();
        }
        if i.consume_key(Modifiers::NONE, Key::U) {
            state.undo();
        }
        if i.consume_key(Modifiers::CTRL, Key::F) {
            state.mode = Mode::AppFilter;
            state.app_search_query.clear();
        }
    });
}

fn handle_insert_mode_input(ctx: &Context, state: &mut AppState) {
    ctx.input_mut(|i| {
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Escape))
            || i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::OpenBracket))
        {
            state.exit_insert_mode(false);
        } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Enter)) {
            state.exit_insert_mode(true);
        }

        for event in &i.events.clone() {
            if let egui::Event::Text(text) = event {
                state.temp_edit_buffer.push_str(text);
            }
        }
        if i.key_pressed(Key::Backspace) {
            state.temp_edit_buffer.pop();
        }
    });
}

fn handle_search_mode_input(ctx: &Context, state: &mut AppState) {
    ctx.input(|i| {
        if i.key_pressed(Key::Escape) {
            state.mode = Mode::Normal;
            state.search_query.clear();
            state.refilter();
        }
        if i.key_pressed(Key::Enter) {
            state.mode = Mode::Normal;
        }
        if i.key_pressed(Key::Backspace) {
            state.search_query.pop();
            state.refilter();
        }
    });
    for event in &ctx.input(|i| i.events.clone()) {
        if let egui::Event::Text(text) = event {
            state.search_query.push_str(text);
            state.refilter();
        }
    }
}

fn handle_command_mode_input(ctx: &Context, state: &mut AppState) {
    ctx.input(|i| {
        if i.key_pressed(Key::Escape) {
            state.mode = Mode::Normal;
        }
        if i.key_pressed(Key::Enter) {
            match state.command_buffer.as_str() {
                "w" => state.save_keybinds(),
                "wq" => {
                    state.save_keybinds();
                    state.should_quit = true;
                }
                "q" => state.should_quit = true,
                "q!" => state.should_quit = true,
                _ => state.status_message = format!("Not a command: {}", state.command_buffer),
            }
            state.mode = Mode::Normal;
        }
        if i.key_pressed(Key::Backspace) {
            state.command_buffer.pop();
        }
    });
    for event in &ctx.input(|i| i.events.clone()) {
        if let egui::Event::Text(text) = event {
            state.command_buffer.push_str(text);
        }
    }
}

fn handle_app_filter_input(ctx: &Context, state: &mut AppState) {
    ctx.input(|i| {
        if i.key_pressed(Key::Escape) {
            state.mode = Mode::Normal;
        }
        if i.key_pressed(Key::Enter) {
            state.mode = Mode::Normal;
        }
        if i.key_pressed(Key::Backspace) {
            state.app_search_query.pop();
        }
    });
    for event in &ctx.input(|i| i.events.clone()) {
        if let egui::Event::Text(text) = event {
            state.app_search_query.push_str(text);
        }
    }
}

fn draw_main_table(ui: &mut Ui, state: &mut AppState) {
    egui::ScrollArea::vertical().show(ui, |ui| {
        egui::Grid::new("keybinds_grid")
            .num_columns(3)
            .spacing([40.0, 4.0])
            .striped(true)
            .show(ui, |ui| {
                ui.label(RichText::new("Keybind").strong());
                ui.label(RichText::new("Description").strong());
                ui.label(RichText::new("Application").strong());
                ui.end_row();

                for (row_idx, &real_idx) in state.filtered_indices.iter().enumerate() {
                    let keybind = &state.keybinds[real_idx];
                    for col_idx in 0..3 {
                        let is_selected =
                            state.selected_cell == (row_idx, col_idx) && state.mode != Mode::Insert;
                        let is_editing =
                            state.selected_cell == (row_idx, col_idx) && state.mode == Mode::Insert;

                        let text_val = match col_idx {
                            0 => {
                                if is_editing && state.is_listening_for_keybind {
                                    "Press key...".to_string()
                                } else {
                                    keybind.keys.clone()
                                }
                            }
                            1 => {
                                if is_editing {
                                    state.temp_edit_buffer.clone()
                                } else {
                                    keybind.description.clone()
                                }
                            }
                            2 => {
                                if is_editing {
                                    state.temp_edit_buffer.clone()
                                } else {
                                    keybind.application.clone()
                                }
                            }
                            _ => String::new(),
                        };

                        let text_widget = RichText::new(text_val).monospace();
                        let sense = if is_selected || is_editing {
                            Sense::click()
                        } else {
                            Sense::hover()
                        };
                        let label = ui.add(egui::Label::new(text_widget).sense(sense));

                        if is_selected {
                            ui.painter().rect_stroke(
                                label.rect,
                                3.0,
                                egui::Stroke::new(2.0, egui::Color32::from_rgb(100, 150, 255)),
                            );
                        }
                    }
                    ui.end_row();
                }
            });
    });
}

fn draw_status_bar(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        let mode_text = match state.mode {
            Mode::Normal => "-- NORMAL --",
            Mode::Insert => "-- INSERT --",
            Mode::Search => "/",
            Mode::Command => ":",
            Mode::AppFilter => "Filter Apps:",
        };
        ui.label(RichText::new(mode_text).strong().monospace());

        let display_text = match state.mode {
            Mode::Search => &state.search_query,
            Mode::Command => &state.command_buffer,
            Mode::AppFilter => &state.app_search_query,
            _ => &state.status_message,
        };
        ui.label(RichText::new(display_text).monospace());

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            let app_filter_text = state.application_filter.as_deref().unwrap_or("ALL");
            ui.label(RichText::new(app_filter_text).strong().monospace());
        });
    });
}

fn draw_app_filter_popup(ctx: &Context, state: &mut AppState) {
    egui::Window::new("Filter by Application")
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, -100.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            ui.label("Type to filter applications. Press Enter to select, Esc to cancel.");
            ui.text_edit_singleline(&mut state.app_search_query)
                .request_focus();
            ui.separator();

            let mut apps: HashSet<String> = state
                .keybinds
                .iter()
                .map(|kb| kb.application.clone())
                .collect();
            let mut sorted_apps: Vec<String> = apps.drain().collect();
            sorted_apps.sort();

            let matcher = SkimMatcherV2::default();
            let filtered_apps: Vec<String> = sorted_apps
                .into_iter()
                .filter(|app| {
                    matcher.fuzzy_match(app, &state.app_search_query).is_some()
                        || state.app_search_query.is_empty()
                })
                .collect();

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    if ui.button("Clear Filter").clicked() {
                        state.application_filter = None;
                        state.mode = Mode::Normal;
                        state.refilter();
                    }
                    for app in filtered_apps {
                        if ui.button(&app).clicked() {
                            state.application_filter = Some(app);
                            state.mode = Mode::Normal;
                            state.refilter();
                            break;
                        }
                    }
                });
        });
}
