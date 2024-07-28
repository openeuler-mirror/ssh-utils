use anyhow::{Context, Result};
use argon2::Config;
use openssl::symm::{Cipher, Crypter, Mode};
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};

/**
    derive 16 bytes digest from password
*/
pub fn derive_sha256_digest(password: &str) -> [u8; 16] {
    let mut hasher = Sha256::new();
    hasher.update(password.as_bytes());
    let result = hasher.finalize();
    let mut salt = [0u8; 16];
    salt.copy_from_slice(&result[..16]);
    salt
}

/**
    derive 32 bytes hash key from password by argon2
*/
pub fn derive_key_from_password(password: &str) -> Result<[u8; 32]> {
    // Step 1: Derive a 16-byte SHA-256 digest from the password.
    let salt = derive_sha256_digest(password);

    let config = Config::default();
    let key = argon2::hash_raw(password.as_bytes(), &salt, &config)
        .context("Failed to derive key using Argon2")?;
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(&key);
    Ok(key_arr)
}

/**
    generate random iv
*/
pub fn generate_iv() -> [u8; 16] {
    let mut iv = [0u8; 16];
    let mut rng = thread_rng();
    rng.fill(&mut iv);
    iv
}

pub fn aes_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    let cipher = Cipher::aes_256_ctr();
    let mut crypter =
        Crypter::new(cipher, Mode::Encrypt, key, Some(iv)).context("Failed to create Crypter")?;
    let mut ciphertext = vec![0; data.len() + cipher.block_size()];
    let mut count = crypter
        .update(data, &mut ciphertext)
        .context("Failed to encrypt data")?;
    count += crypter
        .finalize(&mut ciphertext[count..])
        .context("Failed to finalize encryption")?;
    ciphertext.truncate(count);
    Ok(ciphertext)
}

pub fn aes_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>> {
    // Step: Decrypt the data using AES-256-CTR
    let cipher = Cipher::aes_256_ctr();
    let mut crypter =
        Crypter::new(cipher, Mode::Decrypt, key, Some(iv)).context("Failed to create Crypter")?;
    let mut plaintext = vec![0; data.len() + cipher.block_size()];
    let mut count = crypter
        .update(data, &mut plaintext)
        .context("Failed to decrypt data")?;
    count += crypter
        .finalize(&mut plaintext[count..])
        .context("Failed to finalize decryption")?;
    plaintext.truncate(count);
    Ok(plaintext)
}