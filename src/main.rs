use directories::ProjectDirs;
use eframe::egui;
use egui::{
    vec2, Color32, Context, Frame, Key, KeyboardShortcut, Layout, Modifiers, RichText, TextFormat,
    Ui,
};
use fuzzy_matcher::skim::SkimMatcherV2;
use fuzzy_matcher::FuzzyMatcher;
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use toml;

const MAX_UNDO_HISTORY: usize = 20;

#[derive(Serialize, Deserialize, Clone, Debug, PartialEq, Eq, Hash)]
struct Keybind {
    keys: String,
    description: String,
    application: String,
}

#[derive(Serialize, Deserialize, Clone, Debug)]
struct KeybindEntry {
    keys: String,
    description: String,
}

#[derive(Serialize, Deserialize, Debug)]
struct AppKeybinds {
    application: String,
    keybinds: Vec<KeybindEntry>,
}

struct FilteredItem {
    original_index: usize,
    match_indices: Option<Vec<usize>>,
}

#[derive(PartialEq, Clone, Copy, Debug)]
enum Mode {
    Normal,
    Insert,
    Search,
    Command,
    AppFilter,
    Export,
    Import,
    Help,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
struct Keymap {
    up: String,
    down: String,
    left: Vec<String>,
    right: Vec<String>,
    goto_top: String,
    goto_bottom: String,
    insert_mode: String,
    normal_mode: String,
    normal_mode_alt: String,
    search_mode: String,
    command_mode: String,
    undo: String,
    delete_line: String,
    delete_leader: String,
    new_line_below: String,
    new_line_above: String,
    app_filter: String,
    export_menu: String,
    import_menu: String,
    leader: String,
}

impl Default for Keymap {
    fn default() -> Self {
        Self {
            up: "K".into(),
            down: "J".into(),
            left: vec!["H".into(), "B".into()],
            right: vec!["L".into(), "W".into(), "E".into()],
            goto_top: "G".into(),
            goto_bottom: "G".into(), // Special case for Shift+G
            insert_mode: "I".into(),
            normal_mode: "Escape".into(),
            normal_mode_alt: "Control+OpenBracket".into(),
            search_mode: "Slash".into(),
            command_mode: "Colon".into(),
            undo: "U".into(),
            delete_line: "D".into(), // For 'dd'
            delete_leader: "D".into(),
            new_line_below: "O".into(),
            new_line_above: "O".into(), // Special case for Shift+O
            app_filter: "F".into(),
            export_menu: "E".into(),
            import_menu: "I".into(),
            leader: "Space".into(),
        }
    }
}

// This function correctly maps a string from config to an egui::Key
fn string_to_key(s: &str) -> Option<Key> {
    Some(match s.to_uppercase().as_str() {
        "DOWN" => Key::ArrowDown,
        "LEFT" => Key::ArrowLeft,
        "RIGHT" => Key::ArrowRight,
        "UP" => Key::ArrowUp,
        "ESCAPE" => Key::Escape,
        "TAB" => Key::Tab,
        "BACKSPACE" => Key::Backspace,
        "ENTER" => Key::Enter,
        "SPACE" => Key::Space,
        "INSERT" => Key::Insert,
        "DELETE" => Key::Delete,
        "HOME" => Key::Home,
        "END" => Key::End,
        "PAGEDOWN" => Key::PageDown,
        "PAGEUP" => Key::PageUp,
        "A" => Key::A,
        "B" => Key::B,
        "C" => Key::C,
        "D" => Key::D,
        "E" => Key::E,
        "F" => Key::F,
        "G" => Key::G,
        "H" => Key::H,
        "I" => Key::I,
        "J" => Key::J,
        "K" => Key::K,
        "L" => Key::L,
        "M" => Key::M,
        "N" => Key::N,
        "O" => Key::O,
        "P" => Key::P,
        "Q" => Key::Q,
        "R" => Key::R,
        "S" => Key::S,
        "T" => Key::T,
        "U" => Key::U,
        "V" => Key::V,
        "W" => Key::W,
        "X" => Key::X,
        "Y" => Key::Y,
        "Z" => Key::Z,
        "F1" => Key::F1,
        "F2" => Key::F2,
        "F3" => Key::F3,
        "F4" => Key::F4,
        "F5" => Key::F5,
        "F6" => Key::F6,
        "F7" => Key::F7,
        "F8" => Key::F8,
        "F9" => Key::F9,
        "F10" => Key::F10,
        "F11" => Key::F11,
        "F12" => Key::F12,
        "SLASH" => Key::Slash,
        "COLON" => Key::Colon,
        "SEMICOLON" => Key::Semicolon,
        _ => return None,
    })
}

struct AppState {
    keybinds: Vec<Keybind>,
    all_applications: HashSet<String>,
    filtered_items: Vec<FilteredItem>,
    selected_cell: (usize, usize),
    mode: Mode,
    search_query: String,
    command_buffer: String,
    status_message: String,
    current_application: String,
    app_search_query: String,
    temp_edit_buffer: String,
    is_listening_for_keybind: bool,
    should_quit: bool,
    undo_history: Vec<Vec<Keybind>>,
    ignore_next_input_frame: bool,
    app_filter_selected_index: usize,
    leader_key_pressed: bool,
    delete_leader_pressed: bool,
    just_created_new_keybind: bool,
    dirty: bool,
    debug_mode: bool,
    keymap: Keymap,
}

fn get_config_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "ctrlset", "ctrlset") {
        proj_dirs.config_dir().to_path_buf()
    } else {
        PathBuf::from(".")
    }
}

fn get_data_dir() -> PathBuf {
    if let Some(proj_dirs) = ProjectDirs::from("com", "ctrlset", "ctrlset") {
        proj_dirs.data_dir().to_path_buf()
    } else {
        PathBuf::from(".")
    }
}

fn load_or_create_config() -> Keymap {
    let config_dir = get_config_dir();
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir)
            .unwrap_or_else(|e| eprintln!("Failed to create config dir: {}", e));
    }
    let config_path = config_dir.join("config.toml");

    if !config_path.exists() {
        let default_keymap = Keymap::default();
        let toml_string =
            toml::to_string_pretty(&default_keymap).expect("Could not serialize default keymap");
        fs::write(&config_path, toml_string)
            .unwrap_or_else(|e| eprintln!("Failed to write default config: {}", e));
        return default_keymap;
    }

    let toml_string = fs::read_to_string(config_path).unwrap_or_default();
    toml::from_str(&toml_string).unwrap_or_else(|e| {
        eprintln!("Failed to parse config.toml, using defaults. Error: {}", e);
        Keymap::default()
    })
}

impl AppState {
    fn new(debug_mode: bool) -> Self {
        let keymap = load_or_create_config();
        let mut app = Self {
            keybinds: vec![],
            all_applications: HashSet::new(),
            filtered_items: vec![],
            selected_cell: (0, 0),
            mode: Mode::Normal,
            search_query: String::new(),
            command_buffer: String::new(),
            status_message: "Welcome to ctrlset!".to_string(),
            current_application: String::new(),
            app_search_query: String::new(),
            temp_edit_buffer: String::new(),
            is_listening_for_keybind: false,
            should_quit: false,
            undo_history: Vec::new(),
            ignore_next_input_frame: false,
            app_filter_selected_index: 0,
            leader_key_pressed: false,
            delete_leader_pressed: false,
            just_created_new_keybind: false,
            dirty: false,
            debug_mode,
            keymap,
        };
        app.load_all_keybinds();
        let mut apps: Vec<_> = app.all_applications.iter().cloned().collect();
        apps.sort();
        app.current_application = apps
            .get(0)
            .cloned()
            .unwrap_or_else(|| "default".to_string());
        if !app.all_applications.contains(&app.current_application) {
            app.all_applications.insert(app.current_application.clone());
        }
        app.refilter();
        app
    }

    fn get_all_applications(&self) -> Vec<String> {
        let mut apps: Vec<_> = self.all_applications.iter().cloned().collect();
        apps.sort();
        apps
    }

    fn save_current_app_keybinds(&mut self) {
        let dir = get_data_dir();
        if !dir.exists() {
            if let Err(e) = fs::create_dir_all(&dir) {
                self.status_message = format!("Error creating directory: {}", e);
                return;
            }
        }

        let app_name = &self.current_application;
        let path = dir.join(format!("{}.json", app_name));

        let entries: Vec<KeybindEntry> = self
            .keybinds
            .iter()
            .filter(|kb| &kb.application == app_name)
            .map(|kb| KeybindEntry {
                keys: kb.keys.clone(),
                description: kb.description.clone(),
            })
            .collect();

        let app_keybinds = AppKeybinds {
            application: app_name.clone(),
            keybinds: entries,
        };

        match serde_json::to_string_pretty(&app_keybinds) {
            Ok(json) => {
                if fs::write(&path, json).is_ok() {
                    self.status_message = format!("Saved {} successfully.", app_name);
                    self.dirty = false;
                } else {
                    self.status_message = format!("Error: Failed to write to {}.", path.display());
                }
            }
            Err(_) => {
                self.status_message = "Error: Failed to serialize keybinds.".to_string();
            }
        }
    }

    fn load_all_keybinds(&mut self) {
        self.keybinds.clear();
        self.all_applications.clear();
        let dir = get_data_dir();

        if !dir.exists() {
            if let Err(e) = fs::create_dir_all(&dir) {
                self.status_message = format!(
                    "Failed to create data directory at {}: {}",
                    dir.display(),
                    e
                );
                return;
            }
            self.status_message = format!("Created new data directory at {}.", dir.display());
        }

        match fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries {
                    if let Ok(entry) = entry {
                        let path = entry.path();
                        if path.is_file()
                            && path.extension().and_then(|s| s.to_str()) == Some("json")
                        {
                            if let Ok(data) = fs::read_to_string(&path) {
                                if let Ok(app_keybinds) = serde_json::from_str::<AppKeybinds>(&data)
                                {
                                    self.all_applications
                                        .insert(app_keybinds.application.clone());
                                    for entry in app_keybinds.keybinds {
                                        self.keybinds.push(Keybind {
                                            keys: entry.keys,
                                            description: entry.description,
                                            application: app_keybinds.application.clone(),
                                        });
                                    }
                                }
                            }
                        }
                    }
                }
                if !self.keybinds.is_empty() {
                    self.status_message = "Keybinds loaded.".to_string();
                }
                self.dirty = false;
            }
            Err(_) => {
                self.status_message = "Error reading keybinds directory.".to_string();
            }
        }
    }

    fn refilter(&mut self) {
        let matcher = SkimMatcherV2::default();
        let search_query: String = self
            .search_query
            .chars()
            .filter(|c| !c.is_whitespace())
            .collect::<String>()
            .to_lowercase();
        let current_app = &self.current_application;

        self.filtered_items = self
            .keybinds
            .iter()
            .enumerate()
            .filter_map(|(idx, kb)| {
                if &kb.application != current_app {
                    return None;
                }
                if search_query.is_empty() {
                    Some(FilteredItem {
                        original_index: idx,
                        match_indices: None,
                    })
                } else {
                    let combined_string = format!("{} {}", kb.keys, kb.description);
                    if let Some((_, indices)) =
                        matcher.fuzzy_indices(&combined_string, &search_query)
                    {
                        Some(FilteredItem {
                            original_index: idx,
                            match_indices: Some(indices),
                        })
                    } else {
                        None
                    }
                }
            })
            .collect();
        self.clamp_selection();
    }

    fn clamp_selection(&mut self) {
        let num_rows = self.filtered_items.len();
        if num_rows == 0 {
            self.selected_cell = (0, 0);
        } else {
            self.selected_cell.0 = self.selected_cell.0.min(num_rows.saturating_sub(1));
        }
        self.selected_cell.1 = self.selected_cell.1.min(1);
    }

    fn enter_insert_mode(&mut self) {
        if self.filtered_items.is_empty() && !self.just_created_new_keybind {
            return;
        }
        self.mode = Mode::Insert;
        let (row_idx, col_idx) = self.selected_cell;
        let real_idx = self.filtered_items[row_idx].original_index;

        self.temp_edit_buffer = match col_idx {
            0 => {
                self.is_listening_for_keybind = true;
                self.ignore_next_input_frame = true;
                self.keybinds[real_idx].keys.clone()
            }
            1 => self.keybinds[real_idx].description.clone(),
            _ => String::new(),
        };
    }

    fn exit_insert_mode(&mut self, saved: bool) {
        self.is_listening_for_keybind = false;
        let (row_idx, col_idx) = self.selected_cell;

        if saved {
            if self.just_created_new_keybind && col_idx == 0 {
                self.push_to_undo_history();
            }

            if let Some(item) = self.filtered_items.get(row_idx) {
                let kb = &mut self.keybinds[item.original_index];
                let old_val = match col_idx {
                    0 => &kb.keys,
                    1 => &kb.description,
                    _ => "",
                };
                if old_val != &self.temp_edit_buffer {
                    self.dirty = true;
                }
                match col_idx {
                    0 => kb.keys = self.temp_edit_buffer.clone(),
                    1 => kb.description = self.temp_edit_buffer.clone(),
                    _ => {}
                }
            }

            if self.just_created_new_keybind && col_idx == 0 {
                self.selected_cell.1 = 1;
                self.enter_insert_mode();
                return;
            }
        } else if self.just_created_new_keybind {
            if let Some(item) = self.filtered_items.get(row_idx) {
                let kb = &self.keybinds[item.original_index];
                if kb.keys.is_empty() && kb.description.is_empty() {
                    self.keybinds.remove(item.original_index);
                    self.refilter();
                }
            }
        }

        self.mode = Mode::Normal;
        self.temp_edit_buffer.clear();
        self.just_created_new_keybind = false;
    }

    fn push_to_undo_history(&mut self) {
        if self.undo_history.len() >= MAX_UNDO_HISTORY {
            self.undo_history.remove(0);
        }
        self.undo_history.push(self.keybinds.clone());
        self.dirty = true;
    }

    fn undo(&mut self) {
        if let Some(last_state) = self.undo_history.pop() {
            self.keybinds = last_state;
            self.refilter();
            self.dirty = true;
            self.status_message = "Undo successful.".to_string();
        } else {
            self.status_message = "Nothing to undo.".to_string();
        }
    }
}

fn main() -> Result<(), eframe::Error> {
    let args: Vec<String> = std::env::args().collect();
    let debug_mode = args.contains(&"--debug".to_string());

    let options = eframe::NativeOptions {
        viewport: egui::ViewportBuilder::default().with_inner_size([800.0, 600.0]),
        ..Default::default()
    };
    eframe::run_native(
        "ctrlset",
        options,
        Box::new(move |_cc| Box::new(App::new(debug_mode))),
    )
}

struct App {
    state: AppState,
}
impl App {
    fn new(debug_mode: bool) -> Self {
        Self {
            state: AppState::new(debug_mode),
        }
    }
}

impl eframe::App for App {
    fn update(&mut self, ctx: &egui::Context, _frame: &mut eframe::Frame) {
        let state = &mut self.state;

        let title = if state.dirty {
            "ctrlset [*]"
        } else {
            "ctrlset"
        };
        ctx.send_viewport_cmd(egui::ViewportCommand::Title(title.to_string()));

        if state.is_listening_for_keybind {
            handle_key_capture(ctx, state);
        } else {
            handle_global_input(ctx, state);
        }

        if state.should_quit {
            ctx.send_viewport_cmd(egui::ViewportCommand::Close);
        }

        egui::CentralPanel::default().show(ctx, |ui| {
            ui.with_layout(Layout::top_down(egui::Align::Center), |ui| {
                ui.add_space(20.0);
                Frame::group(ui.style()).show(ui, |ui| {
                    draw_main_table(ui, state);
                });
            });
        });

        egui::TopBottomPanel::bottom("status_bar").show(ctx, |ui| {
            draw_status_bar(ui, state);
        });

        match state.mode {
            Mode::AppFilter => draw_app_filter_popup(ctx, state),
            Mode::Export => draw_export_popup(ctx, state),
            Mode::Import => draw_import_popup(ctx, state),
            Mode::Help => draw_help_popup(ctx, state),
            _ => {}
        }
    }
}

fn handle_key_capture(ctx: &Context, state: &mut AppState) {
    if state.ignore_next_input_frame {
        state.ignore_next_input_frame = false;
        return;
    }

    ctx.input(|i| {
        if i.key_pressed(Key::Escape) {
            state.exit_insert_mode(true);
            return;
        }
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
            if !is_key_just_a_modifier(key) {
                let mut parts = Vec::new();
                let mods = i.modifiers;
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
            | Key::F20
    )
}

fn handle_global_input(ctx: &Context, state: &mut AppState) {
    if state.debug_mode {
        ctx.input(|i| {
            i.events.iter().for_each(|e| {
                if let egui::Event::Key {
                    key,
                    pressed: true,
                    modifiers,
                    ..
                } = e
                {
                    println!("DEBUG: Key pressed: {:?}, Modifiers: {:?}", key, modifiers);
                }
            })
        });
    }

    match state.mode {
        Mode::Normal => handle_normal_mode_input(ctx, state),
        Mode::Insert => handle_insert_mode_input(ctx, state),
        Mode::Search => handle_search_mode_input(ctx, state),
        Mode::Command => handle_command_mode_input(ctx, state),
        Mode::AppFilter | Mode::Export | Mode::Import | Mode::Help => {}
    }
}

fn handle_normal_mode_input(ctx: &Context, state: &mut AppState) {
    ctx.input_mut(|i| {
        let keymap = state.keymap.clone();

        let leader_key = string_to_key(&keymap.leader).unwrap_or(Key::Space);
        let down_key = string_to_key(&keymap.down).unwrap_or(Key::J);
        let up_key = string_to_key(&keymap.up).unwrap_or(Key::K);

        if state.leader_key_pressed {
            let mut consumed = false;
            if i.consume_key(
                Modifiers::NONE,
                string_to_key(&keymap.app_filter).unwrap_or(Key::F),
            ) {
                state.mode = Mode::AppFilter;
                consumed = true;
            } else if i.consume_key(
                Modifiers::NONE,
                string_to_key(&keymap.export_menu).unwrap_or(Key::E),
            ) {
                state.mode = Mode::Export;
                consumed = true;
            } else if i.consume_key(
                Modifiers::NONE,
                string_to_key(&keymap.import_menu).unwrap_or(Key::I),
            ) {
                state.mode = Mode::Import;
                consumed = true;
            }

            if consumed
                || i.events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Key { pressed: true, .. }))
            {
                state.leader_key_pressed = false;
            }
            return;
        }

        if state.delete_leader_pressed {
            let mut consumed_key = false;
            let mut original_indices_to_delete = vec![];
            let current_row = state.selected_cell.0;

            if i.consume_key(
                Modifiers::NONE,
                string_to_key(&keymap.delete_leader).unwrap_or(Key::D),
            ) {
                if let Some(item) = state.filtered_items.get(current_row) {
                    original_indices_to_delete.push(item.original_index);
                }
                consumed_key = true;
            } else if i.consume_key(Modifiers::NONE, down_key) {
                if let Some(item) = state.filtered_items.get(current_row) {
                    original_indices_to_delete.push(item.original_index);
                }
                if let Some(item) = state.filtered_items.get(current_row + 1) {
                    original_indices_to_delete.push(item.original_index);
                }
                consumed_key = true;
            } else if i.consume_key(Modifiers::NONE, up_key) {
                if let Some(item) = state.filtered_items.get(current_row) {
                    original_indices_to_delete.push(item.original_index);
                }
                if current_row > 0 {
                    if let Some(item) = state.filtered_items.get(current_row - 1) {
                        original_indices_to_delete.push(item.original_index);
                    }
                }
                consumed_key = true;
            }

            if !original_indices_to_delete.is_empty() {
                state.push_to_undo_history();
                original_indices_to_delete.sort_unstable();
                original_indices_to_delete.dedup();
                original_indices_to_delete.reverse();

                for index in &original_indices_to_delete {
                    state.keybinds.remove(*index);
                }

                state.status_message =
                    format!("{} keybind(s) deleted.", original_indices_to_delete.len());
                state.refilter();
                state.clamp_selection();
            }

            if consumed_key
                || i.events
                    .iter()
                    .any(|e| matches!(e, egui::Event::Key { pressed: true, .. }))
            {
                state.delete_leader_pressed = false;
            }
            return;
        }

        if !state.leader_key_pressed && !state.delete_leader_pressed {
            if i.consume_key(Modifiers::NONE, leader_key) {
                state.leader_key_pressed = true;
                return;
            }
            if i.consume_key(
                Modifiers::NONE,
                string_to_key(&keymap.delete_leader).unwrap_or(Key::D),
            ) {
                state.delete_leader_pressed = true;
                return;
            }
        }

        if i.consume_key(
            Modifiers::SHIFT,
            string_to_key(&keymap.goto_bottom).unwrap_or(Key::G),
        ) {
            state.selected_cell.0 = state.filtered_items.len().saturating_sub(1);
        }
        if keymap.goto_top == "G" && i.key_pressed(Key::G) && i.key_down(Key::G) {
            state.selected_cell.0 = 0;
        }

        if i.consume_key(Modifiers::NONE, down_key) {
            let num_rows = state.filtered_items.len();
            if num_rows > 0 {
                state.selected_cell.0 = (state.selected_cell.0 + 1).min(num_rows - 1);
            }
        }
        if i.consume_key(Modifiers::NONE, up_key) {
            state.selected_cell.0 = state.selected_cell.0.saturating_sub(1);
        }

        if keymap
            .right
            .iter()
            .any(|k| i.consume_key(Modifiers::NONE, string_to_key(k).unwrap_or(Key::L)))
        {
            state.selected_cell.1 = (state.selected_cell.1 + 1).min(1);
        }
        if keymap
            .left
            .iter()
            .any(|k| i.consume_key(Modifiers::NONE, string_to_key(k).unwrap_or(Key::H)))
        {
            state.selected_cell.1 = state.selected_cell.1.saturating_sub(1);
        }

        if i.consume_key(
            Modifiers::NONE,
            string_to_key(&keymap.insert_mode).unwrap_or(Key::I),
        ) {
            if !state.just_created_new_keybind {
                state.push_to_undo_history();
            }
            state.enter_insert_mode();
        }
        if i.consume_key(
            Modifiers::NONE,
            string_to_key(&keymap.search_mode).unwrap_or(Key::Slash),
        ) {
            state.mode = Mode::Search;
            state.search_query.clear();
        }
        if i.consume_key(Modifiers::SHIFT, Key::Semicolon)
            || i.consume_key(
                Modifiers::NONE,
                string_to_key(&keymap.command_mode).unwrap_or(Key::Colon),
            )
        {
            state.mode = Mode::Command;
            state.command_buffer.clear();
        }

        if i.consume_key(
            Modifiers::NONE,
            string_to_key(&keymap.new_line_below).unwrap_or(Key::O),
        ) || i.consume_key(
            Modifiers::SHIFT,
            string_to_key(&keymap.new_line_above).unwrap_or(Key::O),
        ) {
            let is_shift = i.modifiers.shift;
            let new_kb = Keybind {
                keys: "".into(),
                description: "".into(),
                application: state.current_application.clone(),
            };
            if is_shift {
                let insert_pos = if state.filtered_items.is_empty() {
                    0
                } else {
                    state.filtered_items[state.selected_cell.0].original_index
                };
                state.keybinds.insert(insert_pos, new_kb);
            } else {
                let insert_pos = if state.filtered_items.is_empty() {
                    0
                } else {
                    state.filtered_items[state.selected_cell.0].original_index + 1
                };
                state
                    .keybinds
                    .insert(insert_pos.min(state.keybinds.len()), new_kb);
                if !state.filtered_items.is_empty() {
                    state.selected_cell.0 += 1;
                }
            }
            state.refilter();
            state.selected_cell.1 = 0;
            state.just_created_new_keybind = true;
            state.enter_insert_mode();
        }
        if i.consume_key(
            Modifiers::NONE,
            string_to_key(&keymap.undo).unwrap_or(Key::U),
        ) {
            state.undo();
        }
    });
}

fn handle_insert_mode_input(ctx: &Context, state: &mut AppState) {
    if state.is_listening_for_keybind {
        return;
    }
    ctx.input_mut(|i| {
        if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Escape))
            || i.consume_shortcut(&KeyboardShortcut::new(Modifiers::CTRL, Key::OpenBracket))
        {
            state.exit_insert_mode(true);
        } else if i.consume_shortcut(&KeyboardShortcut::new(Modifiers::NONE, Key::Enter)) {
            state.exit_insert_mode(true);
        }
    });
}

fn handle_search_mode_input(ctx: &Context, state: &mut AppState) {
    ctx.input(|i| {
        if i.key_pressed(Key::Escape) {
            state.mode = Mode::Normal;
            state.search_query.clear();
            state.refilter();
        } else if i.key_pressed(Key::Enter) {
            state.mode = Mode::Normal;
        } else if i.key_pressed(Key::Backspace) {
            if state.search_query.is_empty() {
                state.mode = Mode::Normal;
            } else {
                state.search_query.pop();
            }
            state.refilter();
        }
    });

    let mut query_changed = false;
    for event in &ctx.input(|i| i.events.clone()) {
        if let egui::Event::Text(text) = event {
            state.search_query.push_str(text);
            query_changed = true;
        }
    }
    if query_changed {
        state.refilter();
    }
}

fn handle_command_mode_input(ctx: &Context, state: &mut AppState) {
    ctx.input(|i| {
        if i.key_pressed(Key::Escape) {
            state.mode = Mode::Normal;
            state.command_buffer.clear();
        }
        if i.key_pressed(Key::Enter) {
            let parts: Vec<&str> = state.command_buffer.split_whitespace().collect();
            let mut command_finished = true;
            match parts.as_slice() {
                ["w"] => state.save_current_app_keybinds(),
                ["wq"] => {
                    state.save_current_app_keybinds();
                    state.should_quit = true;
                }
                ["q"] => {
                    if state.dirty {
                        state.status_message =
                            "Unsaved changes! Use :q! to force quit.".to_string();
                    } else {
                        state.should_quit = true;
                    }
                }
                ["q!"] => state.should_quit = true,
                ["help"] => {
                    state.mode = Mode::Help;
                    command_finished = false;
                }
                ["new", app_name @ ..] => {
                    let app_name_str = app_name.join(" ");
                    if !app_name_str.is_empty() && !state.all_applications.contains(&app_name_str) {
                        state.all_applications.insert(app_name_str.clone());
                        state.current_application = app_name_str;
                        state.refilter();
                        state.dirty = true;
                        state.status_message =
                            format!("Created new app '{}'.", state.current_application);
                    } else {
                        state.status_message = "App name invalid or already exists.".to_string();
                    }
                }
                _ => state.status_message = format!("Not a command: {}", state.command_buffer),
            }

            if command_finished {
                state.mode = Mode::Normal;
            }
            state.command_buffer.clear();
        }
    });
}

fn draw_main_table(ui: &mut Ui, state: &mut AppState) {
    egui::Grid::new("keybinds_grid")
        .num_columns(2)
        .spacing([10.0, 4.0])
        .striped(true)
        .show(ui, |ui| {
            ui.label(RichText::new("Keybind").strong());
            ui.label(RichText::new("Description").strong());
            ui.end_row();

            ui.add_sized([ui.available_width(), 0.0], egui::Label::new(""));
            ui.end_row();

            let items = state
                .filtered_items
                .iter()
                .enumerate()
                .map(|(i, item)| {
                    let keybind = &state.keybinds[item.original_index];
                    (
                        i,
                        keybind.keys.clone(),
                        keybind.description.clone(),
                        item.match_indices.clone(),
                    )
                })
                .collect::<Vec<_>>();

            for (row_idx, keys, description, match_indices) in items {
                // --- Keybind Column ---
                let is_selected = state.selected_cell == (row_idx, 0);
                let is_editing = is_selected && state.mode == Mode::Insert;

                let response = if is_editing && state.is_listening_for_keybind {
                    ui.label(RichText::new("Press key...").monospace())
                } else {
                    let indices: HashSet<usize> = match_indices
                        .as_ref()
                        .map(|v| v.iter().cloned().collect())
                        .unwrap_or_default();
                    let job = create_highlighted_layout(keys.to_string(), indices, 0, ui);
                    ui.label(job)
                };
                if is_selected && state.mode != Mode::Insert {
                    ui.painter().rect_stroke(
                        response.rect.expand(2.0),
                        3.0,
                        ui.visuals().selection.stroke,
                    );
                }

                // --- Description Column ---
                let is_selected = state.selected_cell == (row_idx, 1);
                let is_editing = is_selected && state.mode == Mode::Insert;

                let response = if is_editing {
                    let text_edit = egui::TextEdit::singleline(&mut state.temp_edit_buffer)
                        .font(egui::FontId::monospace(14.0))
                        .frame(false);
                    let r = ui.add(text_edit);
                    r.request_focus();
                    r
                } else {
                    let offset = keys.len() + 1;
                    let indices: HashSet<usize> = match_indices
                        .as_ref()
                        .map(|v| v.iter().cloned().collect())
                        .unwrap_or_default();
                    let job =
                        create_highlighted_layout(description.to_string(), indices, offset, ui);
                    ui.label(job)
                };
                if is_selected && state.mode != Mode::Insert {
                    ui.painter().rect_stroke(
                        response.rect.expand(2.0),
                        3.0,
                        ui.visuals().selection.stroke,
                    );
                }

                ui.end_row();
            }
        });
}

fn create_highlighted_layout(
    text: String,
    indices: HashSet<usize>,
    offset: usize,
    ui: &Ui,
) -> egui::text::LayoutJob {
    let mut job = egui::text::LayoutJob::default();
    let theme_visuals = ui.visuals().clone();
    let highlight_color = Color32::from_rgb(255, 255, 0);

    for (i, c) in text.char_indices() {
        let is_match = indices.contains(&(i + offset));
        job.append(
            &c.to_string(),
            0.0,
            TextFormat {
                font_id: egui::FontId::monospace(14.0),
                color: theme_visuals.text_color(),
                background: if is_match {
                    highlight_color
                } else {
                    Color32::TRANSPARENT
                },
                ..Default::default()
            },
        );
    }
    job
}

fn draw_status_bar(ui: &mut Ui, state: &mut AppState) {
    ui.horizontal(|ui| {
        let mode_text = if state.leader_key_pressed {
            "<leader>"
        } else if state.delete_leader_pressed {
            "<delete>"
        } else {
            match state.mode {
                Mode::Normal => "-- NORMAL --",
                Mode::Insert => "-- INSERT --",
                Mode::Search => "/",
                Mode::Command => ":",
                Mode::AppFilter => "Filter Apps:",
                Mode::Export => "Export:",
                Mode::Import => "Import:",
                Mode::Help => "Help:",
            }
        };

        match state.mode {
            Mode::Command => {
                ui.label(RichText::new(":").strong().monospace());
                let text_edit = ui.add(
                    egui::TextEdit::singleline(&mut state.command_buffer)
                        .frame(false)
                        .desired_width(200.0),
                );
                if !text_edit.has_focus() {
                    text_edit.request_focus();
                }
            }
            Mode::Search => {
                ui.label(RichText::new("/").strong().monospace());
                let text_edit = ui.add(
                    egui::TextEdit::singleline(&mut state.search_query)
                        .frame(false)
                        .desired_width(200.0),
                );
                if !text_edit.has_focus() {
                    text_edit.request_focus();
                }
            }
            _ => {
                ui.label(RichText::new(mode_text).strong().monospace());
                if !state.leader_key_pressed && !state.delete_leader_pressed {
                    ui.label(RichText::new(&state.status_message).monospace());
                }
            }
        }

        ui.with_layout(Layout::right_to_left(egui::Align::Center), |ui| {
            ui.label(
                RichText::new(&state.current_application)
                    .strong()
                    .monospace()
                    .color(Color32::LIGHT_BLUE),
            );
        });
    });
}

fn draw_app_filter_popup(ctx: &Context, state: &mut AppState) {
    let mut close_popup = false;
    egui::Window::new("Filter by Application")
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, -100.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                close_popup = true;
            }
            ui.label("Type to search, ↑/↓ to navigate, Enter to select.");

            let text_edit = ui.add(
                egui::TextEdit::singleline(&mut state.app_search_query).hint_text("Search..."),
            );
            if text_edit.changed() {
                state.app_filter_selected_index = 0;
            }
            if !text_edit.has_focus() {
                text_edit.request_focus();
            }
            ui.separator();

            let all_apps = state.get_all_applications();
            let matcher = SkimMatcherV2::default();
            let filtered_apps: Vec<String> = all_apps
                .into_iter()
                .filter(|app| {
                    matcher.fuzzy_match(app, &state.app_search_query).is_some()
                        || state.app_search_query.is_empty()
                })
                .collect();

            egui::ScrollArea::vertical()
                .max_height(200.0)
                .show(ui, |ui| {
                    for (idx, app) in filtered_apps.iter().enumerate() {
                        let is_selected = idx == state.app_filter_selected_index;
                        let label = ui.selectable_label(is_selected, app);
                        if is_selected {
                            ui.painter().rect_stroke(
                                label.rect,
                                3.0,
                                ui.visuals().selection.stroke,
                            );
                        }
                        if label.clicked() {
                            state.current_application = app.clone();
                            close_popup = true;
                            state.refilter();
                        }
                    }
                });

            if ui.input(|i| !filtered_apps.is_empty() && i.key_pressed(Key::ArrowDown)) {
                state.app_filter_selected_index =
                    (state.app_filter_selected_index + 1).min(filtered_apps.len() - 1);
            }
            if ui.input(|i| !filtered_apps.is_empty() && i.key_pressed(Key::ArrowUp)) {
                state.app_filter_selected_index = state.app_filter_selected_index.saturating_sub(1);
            }
            if ui.input(|i| !filtered_apps.is_empty() && i.key_pressed(Key::Enter)) {
                if let Some(selected_app) = filtered_apps.get(state.app_filter_selected_index) {
                    state.current_application = selected_app.clone();
                    state.refilter();
                }
                close_popup = true;
            }
        });
    if close_popup {
        state.mode = Mode::Normal;
        state.app_search_query.clear();
    }
}

fn draw_export_popup(ctx: &Context, state: &mut AppState) {
    let mut close_popup = false;
    egui::Window::new("Export Keybinds")
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, -100.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                close_popup = true;
            }
            ui.label("Choose what to export:");
            ui.separator();

            if ui
                .button(format!("Export '{}' only", state.current_application))
                .clicked()
            {
                let entries: Vec<_> = state
                    .keybinds
                    .iter()
                    .filter(|kb| kb.application == state.current_application)
                    .map(|kb| KeybindEntry {
                        keys: kb.keys.clone(),
                        description: kb.description.clone(),
                    })
                    .collect();
                let app_keybinds = AppKeybinds {
                    application: state.current_application.clone(),
                    keybinds: entries,
                };
                if let Ok(json) = serde_json::to_string_pretty(&app_keybinds) {
                    if let Some(path) = rfd::FileDialog::new()
                        .add_filter("json", &["json"])
                        .set_file_name(&format!("{}.json", state.current_application))
                        .save_file()
                    {
                        if fs::write(path, json).is_ok() {
                            state.status_message = "Export successful.".to_string();
                        } else {
                            state.status_message = "Error: Failed to write to file.".to_string();
                        }
                    }
                }
                close_popup = true;
            }

            if ui.button("Export All").clicked() {
                if let Some(folder) = rfd::FileDialog::new().pick_folder() {
                    for app_name in state.all_applications.iter() {
                        let entries: Vec<_> = state
                            .keybinds
                            .iter()
                            .filter(|kb| &kb.application == app_name)
                            .map(|kb| KeybindEntry {
                                keys: kb.keys.clone(),
                                description: kb.description.clone(),
                            })
                            .collect();
                        let app_keybinds = AppKeybinds {
                            application: app_name.clone(),
                            keybinds: entries,
                        };
                        if let Ok(json) = serde_json::to_string_pretty(&app_keybinds) {
                            let path = folder.join(format!("{}.json", app_name));
                            if fs::write(path, json).is_err() {
                                state.status_message =
                                    format!("Error writing file for {}.", app_name);
                                break;
                            }
                        }
                    }
                    state.status_message = "Export all successful.".to_string();
                }
                close_popup = true;
            }
        });
    if close_popup {
        state.mode = Mode::Normal;
    }
}

fn draw_import_popup(ctx: &Context, state: &mut AppState) {
    let mut close_popup = false;
    egui::Window::new("Import Keybinds")
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, -100.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                close_popup = true;
            }
            ui.label("Select a JSON file to import.");
            ui.separator();

            let import_logic = |replace: bool, state: &mut AppState| {
                if let Some(path) = rfd::FileDialog::new()
                    .add_filter("json", &["json"])
                    .pick_file()
                {
                    if let Ok(data) = fs::read_to_string(path) {
                        if let Ok(imported_app) = serde_json::from_str::<AppKeybinds>(&data) {
                            state.push_to_undo_history();
                            state
                                .all_applications
                                .insert(imported_app.application.clone());

                            if replace {
                                state
                                    .keybinds
                                    .retain(|kb| kb.application != imported_app.application);
                            }

                            let existing_keybinds: HashSet<_> = state
                                .keybinds
                                .iter()
                                .filter(|kb| kb.application == imported_app.application)
                                .cloned()
                                .collect();
                            for entry in imported_app.keybinds {
                                let new_kb = Keybind {
                                    keys: entry.keys,
                                    description: entry.description,
                                    application: imported_app.application.clone(),
                                };
                                if !existing_keybinds.contains(&new_kb) {
                                    state.keybinds.push(new_kb);
                                }
                            }

                            state.dirty = true;
                            state.refilter();
                            state.status_message = "Import successful.".to_string();
                        } else {
                            state.status_message = "Error: Failed to parse JSON file.".to_string();
                        }
                    }
                }
            };

            if ui.button("Import and Merge").clicked() {
                import_logic(false, state);
                close_popup = true;
            }
            if ui.button("Import and Replace").clicked() {
                import_logic(true, state);
                close_popup = true;
            }
        });
    if close_popup {
        state.mode = Mode::Normal;
    }
}

fn draw_help_popup(ctx: &Context, state: &mut AppState) {
    let mut close_popup = false;
    egui::Window::new("Help")
        .anchor(egui::Align2::CENTER_CENTER, vec2(0.0, 0.0))
        .collapsible(false)
        .resizable(false)
        .show(ctx, |ui| {
            if ui.input(|i| i.key_pressed(Key::Escape)) {
                close_popup = true;
            }

            egui::ScrollArea::vertical().show(ui, |ui| {
                ui.heading("Normal Mode");
                egui::Grid::new("help_grid_normal")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("j/k").monospace());
                        ui.label("Move up/down");
                        ui.end_row();
                        ui.label(RichText::new("h/l/b/w/e").monospace());
                        ui.label("Move left/right");
                        ui.end_row();
                        ui.label(RichText::new("gg").monospace());
                        ui.label("Go to top");
                        ui.end_row();
                        ui.label(RichText::new("G").monospace());
                        ui.label("Go to bottom");
                        ui.end_row();
                        ui.label(RichText::new("i").monospace());
                        ui.label("Enter Insert mode");
                        ui.end_row();
                        ui.label(RichText::new("o").monospace());
                        ui.label("Insert new row below");
                        ui.end_row();
                        ui.label(RichText::new("O").monospace());
                        ui.label("Insert new row above");
                        ui.end_row();
                        ui.label(RichText::new("/").monospace());
                        ui.label("Enter Search mode");
                        ui.end_row();
                        ui.label(RichText::new(":").monospace());
                        ui.label("Enter Command mode");
                        ui.end_row();
                        ui.label(RichText::new("u").monospace());
                        ui.label("Undo last change");
                        ui.end_row();
                        ui.label(RichText::new("dd").monospace());
                        ui.label("Delete current row");
                        ui.end_row();
                        ui.label(RichText::new("dj").monospace());
                        ui.label("Delete current and next row");
                        ui.end_row();
                        ui.label(RichText::new("dk").monospace());
                        ui.label("Delete current and previous row");
                        ui.end_row();
                        ui.label(RichText::new("<Space>f").monospace());
                        ui.label("Filter applications");
                        ui.end_row();
                        ui.label(RichText::new("<Space>e").monospace());
                        ui.label("Open export menu");
                        ui.end_row();
                        ui.label(RichText::new("<Space>i").monospace());
                        ui.label("Open import menu");
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.heading("Command Mode");
                egui::Grid::new("help_grid_command")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new(":w").monospace());
                        ui.label("Save current application's keybinds");
                        ui.end_row();
                        ui.label(RichText::new(":wq").monospace());
                        ui.label("Save and quit");
                        ui.end_row();
                        ui.label(RichText::new(":q").monospace());
                        ui.label("Quit (fails if there are unsaved changes)");
                        ui.end_row();
                        ui.label(RichText::new(":q!").monospace());
                        ui.label("Force quit without saving");
                        ui.end_row();
                        ui.label(RichText::new(":new <name>").monospace());
                        ui.label("Create a new application group");
                        ui.end_row();
                        ui.label(RichText::new(":help").monospace());
                        ui.label("Show this help menu");
                        ui.end_row();
                    });

                ui.add_space(10.0);
                ui.heading("Insert/Search/Command Modes");
                egui::Grid::new("help_grid_other")
                    .num_columns(2)
                    .spacing([40.0, 4.0])
                    .show(ui, |ui| {
                        ui.label(RichText::new("Enter").monospace());
                        ui.label("Confirm action");
                        ui.end_row();
                        ui.label(RichText::new("Escape").monospace());
                        ui.label("Cancel action / return to Normal mode");
                        ui.end_row();
                    });
            });

            ui.separator();
            if ui.button("Close").clicked() {
                close_popup = true;
            }
        });
    if close_popup {
        state.mode = Mode::Normal;
    }
}
