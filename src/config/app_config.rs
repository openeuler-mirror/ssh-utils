use anyhow::{Context, Ok};
use serde::Deserialize;
use std::{fs, path::PathBuf};
use anyhow::Result;

#[derive(Deserialize, Debug)]
pub struct Server {
    pub name: String,
    pub ip: String,
    pub user: String,
}

#[derive(Deserialize, Debug, Default)]
pub struct Config {
    pub servers: Vec<Server>,
}

/**
    check if config file and it's directory exists
*/
pub fn ensure_config_exists() -> Result<()> {
    let mut config_dir: PathBuf = if cfg!(debug_assertions) {
        ".".into() // curent running dir
    } else {
        dirs::home_dir().context("Unable to reach user's home directory.")?
    };
    config_dir.push(".config/ssh-utils");

    // Ensure the config directory exists
    if !config_dir.exists() {
        fs::create_dir_all(&config_dir).context(format!(
            "Failed to create config directory at {:?}",
            config_dir
        ))?;
    }

    // Ensure the config file exists
    let config_file_path = config_dir.join("config.toml");
    if !config_file_path.exists() {
        fs::File::create(&config_file_path).context(format!(
            "Failed to create config file at {:?}",
            config_file_path
        ))?;
    }

    Ok(())
}

/**
    read toml format config
    from "~/.config/ssh-utils/config.toml"
*/
pub fn read_config() -> Result<Config> {
    let config_path = if cfg!(debug_assertions) {
        ".config/ssh-utils/config.toml".into()
    } else {
        let mut path = dirs::home_dir().context("Unable to reach user's home directory.")?;
        path.push(".config/ssh-utils/config.toml");
        path
    };

    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Unable to read ssh-utils' config file at {:?}", config_path))?;

    // Check if the config file content is empty
    if config_str.trim().is_empty() {
        return Ok(Config::default());
    }

    let config: Config = toml::from_str(&config_str)
        .context("Failed to parse ssh-utils' config file.")?;

    Ok(config)
}