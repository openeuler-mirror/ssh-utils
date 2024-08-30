use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use uuid::Uuid;
use std::{fs, path::{Path, PathBuf}};

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
    let config_path = get_config_path()?;
    read_config_from_path(&config_path)
}

fn get_config_path() -> Result<PathBuf> {
    if cfg!(debug_assertions) {
        Ok(".config/ssh-utils/config.toml".into())
    } else {
        let mut path = dirs::home_dir().context("Unable to reach user's home directory.")?;
        path.push(".config/ssh-utils/config.toml");
        Ok(path)
    }
}

fn read_config_from_path<P: AsRef<Path>>(config_path: P) -> Result<Config> {
    let config_str = fs::read_to_string(&config_path)
        .with_context(|| format!("Unable to read ssh-utils' config file at {:?}", config_path.as_ref()))?;

    if config_str.trim().is_empty() {
        return Ok(Config::default());
    }

    let config: Config = toml::from_str(&config_str)
        .context("Failed to parse ssh-utils' config file.")?;

    Ok(config)
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;
    use std::fs;

    #[test]
    fn test_read_config_empty_file() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        fs::write(&config_path, "").unwrap();
        let config = read_config_from_path(&config_path).unwrap();
        assert!(config.servers.is_empty());
    }

    #[test]
    fn test_read_config_with_servers() {
        let temp_dir = TempDir::new().unwrap();
        let config_path = temp_dir.path().join("config.toml");
        let config_content = r#"
            [[servers]]
            id = "1"
            name = "Server1"
            ip = "192.168.1.1"
            user = "user1"
            shell = "/bin/bash"
            port = 22

            [[servers]]
            id = "2"
            name = "Server2"
            ip = "192.168.1.2"
            user = "user2"
            shell = "/bin/zsh"
            port = 2222
        "#;
        fs::write(&config_path, config_content).unwrap();

        let config = read_config_from_path(&config_path).unwrap();
        assert_eq!(config.servers.len(), 2);
        assert_eq!(config.servers[0].name, "Server1");
        assert_eq!(config.servers[1].port, 2222);
    }
}