use anyhow::Result;
use anyhow::Context;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use hmac::{Hmac, Mac};
use sha2::Sha256;
use crate::config::crypto::*;

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

/**
    check if config file and it's directory exists
*/
pub fn check_if_vault_bin_exists() -> Result<bool> {
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
