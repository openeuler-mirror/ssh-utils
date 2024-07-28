use anyhow::Result;
use anyhow::{Context, Ok};
use serde::{Deserialize, Serialize};
use std::{path::PathBuf};

use crate::config::crypto::*;

#[derive(Serialize, Deserialize, Debug, PartialEq, Eq)]
pub struct Server {
    pub id: String,
    pub password: String,
}

#[derive(Serialize, Deserialize, Debug, Default, PartialEq, Eq)]
pub struct Vault {
    pub servers: Vec<Server>,
}

/**
    check if config file and it's directory exists
*/
pub fn check_if_password_bin_exists() -> Result<bool> {
    let mut config_dir: PathBuf = if cfg!(debug_assertions) {
        ".".into() // curent running dir
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
pub fn encrypt_vault(vault: &Vault, password: &str) -> Result<Vec<u8>> {
    // Serialize the Vault object to a string.
    let unencrypt_data = toml::to_string(vault).context("Unable to serialize vault to string.")?;

    // Step 1: Derive a 16-byte SHA-256 digest from the password.
    let salt = derive_sha256_digest(password);

    // Step 2: Use argon2 to derive a 32-byte encryption key from the password and salt.
    let encryption_key = derive_key_from_password(password, &salt)?;

    // Step 3: Generate a 16-byte IV (initialization vector).
    let iv = generate_iv();

    // Step 4: Encrypt the serialized Vault data.
    let data = unencrypt_data.as_bytes();
    let encrypted_data = aes_encrypt(&encryption_key, &iv, data)?;

    // Concatenate the IV and encrypted data and return the result.
    let mut result = Vec::with_capacity(iv.len() + encrypted_data.len());
    result.extend_from_slice(&iv);
    result.extend_from_slice(&encrypted_data);

    Ok(result)
}


/**
    decrypt vault
*/
pub fn decrypt_vault(vault: &[u8], password: &str) -> Result<Vault> {
    // Extract the IV and encrypted data.
    let (iv, encrypted_data) = vault.split_at(16);

    // Derive the salt from the password.
    let salt = derive_sha256_digest(password);

    // Derive the encryption key from the password and salt using Argon2.
    let encryption_key = derive_key_from_password(password, &salt)?;

    // Decrypt the data.
    let decrypted_data = aes_decrypt(&encryption_key, iv, encrypted_data)?;

    // Convert the decrypted data to a string and parse it into a Vault object.
    let decrypted_str =
        String::from_utf8(decrypted_data).context("Failed to convert decrypted data to string")?;
    let vault: Vault =
        toml::from_str(&decrypted_str).context("Failed to parse decrypted data as Vault")?;

    Ok(vault)
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
    let encrypt_data = encrypt_vault(&origin_vault,"123")?;
    let decrypt_vault = decrypt_vault(&encrypt_data, "123")?;
    assert_eq!(origin_vault, decrypt_vault);
    Ok(())
}
