use anyhow::Result;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use sha2::Digest;
use std::fs;
use std::path::PathBuf;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use crate::config::crypto::*;
use crate::helper::get_file_path;
use crate::helper::ENCRYPTED_FILE;

type HmacSha256 = Hmac<Sha256>;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Server {
    pub id: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
pub struct Vault {
    pub servers: Vec<Server>,
}

impl Vault {
    pub fn save(&self, encryption_key: &[u8; 32]) -> Result<()> {
        let encrypt_data = encrypt_vault(self, encryption_key)?;
        let file_path = get_file_path(ENCRYPTED_FILE)?;
        
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

        // Write the encrypted data to the file
        fs::write(&file_path, encrypt_data)
            .context(format!("Failed to write encrypted data to file at {:?}", file_path))?;

        Ok(())
    }

    pub fn modify_server(&mut self, id: &str, new_server: Server, encryption_key: &[u8; 32]) -> Result<()> {
        if let Some(server) = self.servers.iter_mut().find(|server| server.id == id) {
            server.password = new_server.password.clone();
            self.save(encryption_key)?;
        } else {
            return Err(anyhow::anyhow!("Server with id {} not found", id));
        }
        Ok(())
    }

    pub fn add_server(&mut self, new_server: Server, encryption_key: &[u8; 32]) -> Result<()> {
        self.servers.push(new_server);
        self.save(encryption_key)?;
        Ok(())
    }

    pub fn delete_server(&mut self, id: &str, encryption_key: &[u8; 32]) -> Result<()> {
        if let Some(pos) = self.servers.iter().position(|server| server.id == id) {
            self.servers.remove(pos);
            self.save(encryption_key)?;
        } else {
            return Err(anyhow::anyhow!("Server with id {} not found", id));
        }
        Ok(())
    }
}

/**
    check if config file and its directory exists
*/
pub fn check_if_vault_bin_exists() -> Result<bool> {
    let mut config_dir: PathBuf = if cfg!(debug_assertions) {
        ".".into() // current running dir
    } else {
        dirs::home_dir().context("Unable to reach user's home directory.")?
    };
    config_dir.push(".config/ssh-utils");

    // check if the encrypted_data file exists
    let config_file_path = config_dir.join("encrypted_data.bin");
    if !config_file_path.exists() {
        return Ok(false);
    }

    Ok(true)
}

/**
    encrypt vault
*/
pub fn encrypt_vault(vault: &Vault, encryption_key: &[u8; 32]) -> Result<Vec<u8>> {
    // Serialize the Vault object to a string.
    let unencrypt_data = toml::to_string(vault).context("Unable to serialize vault to string.")?;

    // Step 3: Generate a 16-byte IV (initialization vector).
    let iv = generate_iv();

    // Step 4: Encrypt the serialized Vault data.
    let data = unencrypt_data.as_bytes();
    let encrypted_data = aes_encrypt(encryption_key, &iv, data)?;

    // Step 5: Compute HMAC for the IV and encrypted data
    let mut mac = HmacSha256::new_from_slice(encryption_key)
        .context("Failed to create HMAC instance")?;
    mac.update(&iv);
    mac.update(&encrypted_data);
    let hmac = mac.finalize().into_bytes();

    // Concatenate the IV, encrypted data, and HMAC and return the result.
    let mut result = Vec::with_capacity(iv.len() + encrypted_data.len() + hmac.len());
    result.extend_from_slice(&iv);
    result.extend_from_slice(&encrypted_data);
    result.extend_from_slice(&hmac);

    Ok(result)
}

/**
    decrypt vault
*/
pub fn decrypt_vault(vault: &[u8], encryption_key: &[u8; 32]) -> Result<Vault> {
    // Extract the IV, encrypted data, and HMAC.
    let (iv, rest) = vault.split_at(16);
    let (encrypted_data, hmac) = rest.split_at(rest.len() - 32);

    // Verify HMAC
    let mut mac = HmacSha256::new_from_slice(encryption_key)
        .context("Failed to create HMAC instance")?;
    mac.update(iv);
    mac.update(encrypted_data);
    mac.verify_slice(hmac).context("HMAC verification failed")?;

    // Decrypt the data.
    let decrypted_data = aes_decrypt(encryption_key, iv, encrypted_data)?;

    // Convert the decrypted data to a string and parse it into a Vault object.
    let decrypted_str =
        String::from_utf8(decrypted_data).context("Failed to convert decrypted data to string")?;
    let vault: Vault =
        toml::from_str(&decrypted_str).context("Failed to parse decrypted data as Vault")?;

    Ok(vault)
}

fn derive_iv_from_id(id: &str) -> [u8; 16] {
    // Step 1: Hash the id using SHA-256.
    let mut hasher = Sha256::new();
    hasher.update(id);
    let result = hasher.finalize();

    // Step 2: Take the first 16 bytes of the hash as the IV.
    let mut iv = [0u8; 16];
    iv.copy_from_slice(&result[..16]);
    iv
}

/**
    encrypt password to string
*/
pub fn encrypt_password(id: &str, password: &str, encryption_key: &[u8; 32]) -> Result<String> {
    // Derive IV from id.
    let iv = derive_iv_from_id(id);

    // Encrypt the password using the provided aes_encrypt function.
    let encrypted_data = aes_encrypt(encryption_key, &iv, password.as_bytes())?;

    // Encode the result as a hex string.
    let encrypted_hex = hex::encode(encrypted_data);

    Ok(encrypted_hex)
}

/**
    decrypt password to string
*/
pub fn decrypt_password(id: &str, encrypted_password: &str, encryption_key: &[u8; 32]) -> Result<String> {
    // Derive IV from id.
    let iv = derive_iv_from_id(id);

    // Decode the encrypted password from hex string.
    let encrypted_data = hex::decode(encrypted_password)
        .context("Failed to decode hex string")?;

    // Decrypt the password using the provided aes_decrypt function.
    let decrypted_data = aes_decrypt(encryption_key, &iv, &encrypted_data)?;

    // Convert the decrypted data to a string.
    let decrypted_password = String::from_utf8(decrypted_data)
        .context("Failed to convert decrypted data to string")?;

    Ok(decrypted_password)
}

#[test]
/**
    test encrypt_password and decrypt_password func
*/
fn test_encryption_decryption_password() -> Result<()> {
    let id = "550e8400-e29b-41d4-a716-446655440000";
    let password = "my_secure_password";
    let encryption_key = derive_key_from_password("123")?;

    let encrypted_password = encrypt_password(id, password, &encryption_key)?;
    println!("Encrypted password: {}", encrypted_password);

    let decrypted_password = decrypt_password(id, &encrypted_password, &encryption_key)?;
    println!("Decrypted password: {}", decrypted_password);

    assert_eq!(password,decrypted_password.as_str());
    Ok(())
}

/**
    test encrypt_vault and decrypt_vault func
*/
#[test]
fn test_encryption_decryption_vault() -> Result<()> {
    let pass_data = r#"
[[servers]]
id = "server1"
password = "secret_password1"

[[servers]]
id = "server2"
password = "secret_password2"
    "#;
    let origin_vault: Vault = toml::from_str(pass_data)?;
    let encryption_key = derive_key_from_password("123")?;
    let encrypt_data = encrypt_vault(&origin_vault, &encryption_key)?;
    let decrypt_vault = match decrypt_vault(&encrypt_data, &encryption_key) {
        Err(e) => {
            if let Some(_) = e.downcast_ref::<hmac::digest::MacError>() {
                println!("wrong password");
                return Err(e);
            } else {
                println!("An unexpected error occurred: {}", e);
                return Err(e);
            }
        },
        Ok(o) => o
    };
    assert_eq!(origin_vault, decrypt_vault);
    Ok(())
}
