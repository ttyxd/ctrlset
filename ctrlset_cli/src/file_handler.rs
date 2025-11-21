use crate::{AppError, Keybind};
use directories::ProjectDirs;
use std::fs::{self, File};
use std::io::{Read, Write};
use std::path::PathBuf;

pub fn get_config_dir() -> Result<PathBuf, AppError> {
    let proj_dirs = ProjectDirs::from("com", "ctrlset", "ctrlset")
        .ok_or(AppError::ConfigDirNotFound)?;
    let config_dir = proj_dirs.config_dir();
    fs::create_dir_all(config_dir)?;
    Ok(config_dir.to_path_buf())
}

pub fn get_config_file() -> Result<PathBuf, AppError> {
    let config_dir = get_config_dir()?;
    Ok(config_dir.join("keybinds.json"))
}

pub fn load_keybinds() -> Result<Vec<Keybind>, AppError> {
    let config_file = get_config_file()?;
    if !config_file.exists() {
        return Ok(Vec::new());
    }
    let mut file = File::open(config_file)?;
    let mut contents = String::new();
    file.read_to_string(&mut contents)?;
    let keybinds: Vec<Keybind> = serde_json::from_str(&contents)?;
    Ok(keybinds)
}

pub fn save_keybinds(keybinds: &[Keybind]) -> Result<(), AppError> {
    let config_file = get_config_file()?;
    let contents = serde_json::to_string_pretty(keybinds)?;
    let mut file = File::create(config_file)?;
    file.write_all(contents.as_bytes())?;
    Ok(())
}
