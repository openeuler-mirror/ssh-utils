use std::path::PathBuf;
use anyhow::{Context, Result};

pub static CONFIG_FILE: &str = "config.toml";
pub static ENCRYPTED_FILE: &str = "encrypted_data.bin";

pub fn get_file_path(file_name: &str) -> Result<String> {
    let mut config_dir: PathBuf = if cfg!(debug_assertions) {
        ".".into() // current running dir
    } else {
        dirs::home_dir().context("Unable to reach user's home directory.")?
    };

    config_dir.push(file_name);
    let file_path = config_dir.to_str().context("Failed to convert path to string")?.to_string();
    Ok(file_path)
}