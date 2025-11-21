pub mod file_handler;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};
use std::collections::HashSet;
use std::fs;
use std::path::PathBuf;
use thiserror::Error;
use file_handler::{load_keybinds, save_keybinds};
use fuzzy_matcher::FuzzyMatcher;
use fuzzy_matcher::skim::SkimMatcherV2;

#[derive(Error, Debug)]
pub enum AppError {
    #[error("I/O error: {0}")]
    Io(#[from] std::io::Error),
    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("Could not find config directory")]
    ConfigDirNotFound,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Keybind {
    pub name: String,
    pub key: String,
    pub app_name: String,
    pub tags: Vec<String>,
}

impl Keybind {
    pub fn new(name: &str, key: &str, app_name: &str) -> Self {
        Self {
            name: name.to_string(),
            key: key.to_string(),
            app_name: app_name.to_string(),
            tags: Vec::new(),
        }
    }

    pub fn name(&self) -> &str {
        &self.name
    }

    pub fn key(&self) -> &str {
        &self.key
    }

    pub fn app_name(&self) -> &str {
        &self.app_name
    }
}

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Add a new keybind
    Add {
        #[arg(short, long)]
        name: String,
        #[arg(short, long)]
        key: String,
        #[arg(short, long)]
        app_name: String,
    },
    /// List all keybinds
    List {
        #[arg(short, long)]
        app_name: Option<String>,
        #[arg(long)]
        json: bool,
    },
    /// Search for keybinds
    Search {
        pattern: String,
        #[arg(short, long)]
        fuzzy: bool,
        #[arg(long)]
        json: bool,
    },
    /// Remove a keybind
    Remove {
        name: String,
    },
    /// Modify a keybind
    Modify {
        name: String,
        #[arg(short, long)]
        key: Option<String>,
        #[arg(short, long)]
        app_name: Option<String>,
    },
    /// List all applications
    Apps,
    /// Import keybinds from a JSON file
    Import {
        file_path: PathBuf,
    },
    /// Export keybinds to a JSON file
    Export {
        file_path: PathBuf,
        #[arg(short, long)]
        app_name: Option<String>,
    },
}

pub fn run() -> anyhow::Result<()> {
    let cli = Cli::parse();

    match &cli.command {
        Commands::Add {
            name,
            key,
            app_name,
        } => {
            let mut keybinds = load_keybinds()?;
            let new_keybind = Keybind::new(name, key, app_name);
            keybinds.push(new_keybind);
            save_keybinds(&keybinds)?;
            println!("Keybind added successfully.");
        }
        Commands::List { app_name, json } => {
            let keybinds = load_keybinds()?;
            let keybinds_to_list: Vec<_> = if let Some(app) = app_name {
                keybinds
                    .into_iter()
                    .filter(|kb| kb.app_name.to_lowercase() == app.to_lowercase())
                    .collect()
            } else {
                keybinds
            };

            if *json {
                println!("{}", serde_json::to_string_pretty(&keybinds_to_list)?);
            } else if keybinds_to_list.is_empty() {
                println!("No keybinds found.");
            } else {
                for keybind in keybinds_to_list {
                    println!(
                        "App: {}, Name: {}, Key: {}",
                        keybind.app_name, keybind.name, keybind.key
                    );
                }
            }
        }
        Commands::Search { pattern, fuzzy, json } => {
            let keybinds = load_keybinds()?;
            if *fuzzy {
                let matcher = SkimMatcherV2::default();
                let mut results: Vec<_> = keybinds
                    .into_iter()
                    .filter_map(|kb| {
                        let name_score = matcher.fuzzy_match(&kb.name, &pattern);
                        let key_score = matcher.fuzzy_match(&kb.key, &pattern);
                        let score = name_score.unwrap_or(0).max(key_score.unwrap_or(0));
                        if score > 0 {
                            Some((kb, score))
                        } else {
                            None
                        }
                    })
                    .collect();
                results.sort_by(|a, b| b.1.cmp(&a.1));

                if *json {
                    let keybinds: Vec<_> = results.into_iter().map(|(kb, _)| kb).collect();
                    println!("{}", serde_json::to_string_pretty(&keybinds)?);
                } else if results.is_empty() {
                    println!("No keybinds found matching '{}'", pattern);
                } else {
                    for (keybind, score) in results {
                        println!(
                            "App: {}, Name: {}, Key: {} (Score: {})",
                            keybind.app_name, keybind.name, keybind.key, score
                        );
                    }
                }
            } else {
                let lower_pattern = pattern.to_lowercase();
                let results: Vec<_> = keybinds
                    .into_iter()
                    .filter(|kb| {
                        kb.name.to_lowercase().contains(&lower_pattern)
                            || kb.key.to_lowercase().contains(&lower_pattern)
                    })
                    .collect();

                if *json {
                    println!("{}", serde_json::to_string_pretty(&results)?);
                } else if results.is_empty() {
                    println!("No keybinds found matching '{}'", pattern);
                } else {
                    for keybind in results {
                        println!(
                            "App: {}, Name: {}, Key: {}",
                            keybind.app_name, keybind.name, keybind.key
                        );
                    }
                }
            }
        }
        Commands::Remove { name } => {
            let mut keybinds = load_keybinds()?;
            let initial_len = keybinds.len();
            keybinds.retain(|kb| kb.name.to_lowercase() != name.to_lowercase());

            if keybinds.len() < initial_len {
                save_keybinds(&keybinds)?;
                println!("Keybind '{}' removed successfully.", name);
            } else {
                println!("Keybind '{}' not found.", name);
            }
        }
        Commands::Modify {
            name,
            key,
            app_name,
        } => {
            let mut keybinds = load_keybinds()?;
            let mut found = false;
            for kb in &mut keybinds {
                if kb.name.to_lowercase() == name.to_lowercase() {
                    if let Some(new_key) = key {
                        kb.key = new_key.clone();
                    }
                    if let Some(new_app_name) = app_name {
                        kb.app_name = new_app_name.clone();
                    }
                    found = true;
                    break;
                }
            }

            if found {
                save_keybinds(&keybinds)?;
                println!("Keybind '{}' modified successfully.", name);
            } else {
                println!("Keybind '{}' not found.", name);
            }
        }
        Commands::Apps => {
            let keybinds = load_keybinds()?;
            let apps: HashSet<_> = keybinds.iter().map(|kb| &kb.app_name).collect();
            if apps.is_empty() {
                println!("No applications found.");
            } else {
                for app in apps {
                    println!("{}", app);
                }
            }
        }
        Commands::Import { file_path } => {
            let mut keybinds = load_keybinds()?;
            let file_contents = fs::read_to_string(file_path)?;
            let imported_keybinds: Vec<Keybind> = serde_json::from_str(&file_contents)?;
            keybinds.extend(imported_keybinds);
            save_keybinds(&keybinds)?;
            println!("Keybinds imported successfully.");
        }
        Commands::Export {
            file_path,
            app_name,
        } => {
            let keybinds = load_keybinds()?;
            let keybinds_to_export: Vec<_> = if let Some(app) = app_name {
                keybinds
                    .into_iter()
                    .filter(|kb| kb.app_name.to_lowercase() == app.to_lowercase())
                    .collect()
            } else {
                keybinds
            };
            let contents = serde_json::to_string_pretty(&keybinds_to_export)?;
            fs::write(file_path, contents)?;
            println!("Keybinds exported successfully.");
        }
    }

    Ok(())
}
