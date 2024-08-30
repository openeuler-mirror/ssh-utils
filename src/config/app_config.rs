use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::{fs, path::PathBuf};

use crate::helper::{get_file_path, CONFIG_FILE};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Server {
    pub id: String,
    pub name: String,
    pub ip: String,
    pub user: String,
    pub shell: String,
    pub port: u16,
}

impl Server {
    pub fn new(name: String, ip: String, user: String, shell: String, port: u16) -> Self {
        Self {
            id: Uuid::new_v4().to_string(),
            name,
            ip,
            user,
            shell,
            port,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Default)]
pub struct Config {
    pub servers: Vec<Server>,
}

impl Config {
    /**
        Save the current config to the specified file.
    */
    pub fn save(&self) -> Result<()> {
        let file_path = get_file_path(CONFIG_FILE)?;
        let config_str = toml::to_string(self).context("Failed to serialize config to TOML.")?;
        
        // Ensure the directory exists
        let path = PathBuf::from(&file_path);
        if let Some(parent) = path.parent() {
            if !parent.exists() {
                fs::create_dir_all(parent).context(format!(
                    "Failed to create config directory at {:?}",
                    parent
                ))?;
            }
        }

        // Write the config to the file
        fs::write(&file_path, config_str)
            .context(format!("Failed to write config to file at {:?}", file_path))?;

        Ok(())
    }

    /**
        Modify a server's information and save the config.
    */
    pub fn modify_server(&mut self, id: &str, new_server: Server) -> Result<()> {
        if let Some(server) = self.servers.iter_mut().find(|server| server.id == id) {
            server.name = new_server.name.clone();
            server.ip = new_server.ip.clone();
            server.user = new_server.user.clone();
            server.shell = new_server.shell.clone();
            server.port = new_server.port;
            self.save()?;
        } else {
            return Err(anyhow::anyhow!("Server with id {} not found", id));
        }
        Ok(())
    }

    /**
        Add a new server and save the config.
    */
    pub fn add_server(&mut self, new_server: Server) -> Result<()> {
        self.servers.push(new_server);
        self.save()?;
        Ok(())
    }

    /**
        Delete a server by id and save the config.
    */
    pub fn delete_server(&mut self, id: &str) -> Result<()> {
        if let Some(pos) = self.servers.iter().position(|server| server.id == id) {
            self.servers.remove(pos);
            self.save()?;
        } else {
            return Err(anyhow::anyhow!("Server with id {} not found", id));
        }
        Ok(())
    }
}

/**
    check if config file and it's directory exists
    if not exists, create them
*/
pub fn ensure_config_exists() -> Result<()> {
    let mut config_dir: PathBuf = if cfg!(debug_assertions) {
        ".".into() // current running dir
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
