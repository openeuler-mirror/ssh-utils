use std::path::PathBuf;
use anyhow::{Context, Result};

use crate::config::app_vault::EncryptionKey;

pub static CONFIG_FILE: &str = "config.toml";
pub static ENCRYPTED_FILE: &str = "encrypted_data.bin";

pub fn get_file_path(file_name: &str) -> Result<String> {
    let mut config_dir: PathBuf = if cfg!(debug_assertions) {
        ".".into() // current running dir
    } else {
        dirs::home_dir().context("Unable to reach user's home directory.")?
    };

    config_dir.push(".config/ssh-utils");
    config_dir.push(file_name);
    let file_path = config_dir.to_str().context("Failed to convert path to string")?.to_string();
    Ok(file_path)
}

pub fn convert_to_array(vec: &EncryptionKey) -> Result<[u8; 32]> {
    let slice = vec.as_slice();
    let array: &[u8; 32] = slice.try_into().context("Failed to convert Vec<u8> to [u8; 32]")?;
    Ok(*array)
}