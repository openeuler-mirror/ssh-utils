use anyhow::{Context, Result};
use argon2::Config;
use openssl::symm::{Cipher, Crypter, Mode};
use rand::{thread_rng, Rng};
use sha2::{Digest, Sha256};
use std::fs::File;
use std::io::{Read, Write};

/**
    derive 16 bytes digest from password
*/
fn derive_sha256_digest(password: &str) -> [u8; 16] {
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
fn derive_key_from_password(password: &str, salt: &[u8]) -> Result<[u8; 32]> {
    let config = Config::default();
    let key = argon2::hash_raw(password.as_bytes(), salt, &config)
        .context("Failed to derive key using Argon2")?;
    let mut key_arr = [0u8; 32];
    key_arr.copy_from_slice(&key);
    Ok(key_arr)
}

/**
    generate random iv
*/
fn generate_iv() -> [u8; 16] {
    let mut iv = [0u8; 16];
    let mut rng = thread_rng();
    rng.fill(&mut iv);
    iv
}

fn aes_encrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>> {
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

fn aes_decrypt(key: &[u8], iv: &[u8], data: &[u8]) -> Result<Vec<u8>> {
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encryption_decryption() -> Result<()> {
        let password = "test_password";

        // Step 1: Derive 16-byte SHA-256 digest
        let salt = derive_sha256_digest(password);
        println!("Derived salt: {:?}", salt);

        // Step 2: Derive 32-byte encryption key using argon2
        let encryption_key = derive_key_from_password(password, &salt)?;
        println!("Derived encryption key: {:?}", encryption_key);

        // Step 3: Generate 16-byte IV
        let iv = generate_iv();
        println!("Generated IV: {:?}", iv);

        // Step 4: Encrypt the string "hello world"
        let data = "hello world".as_bytes();
        let encrypted_data = aes_encrypt(&encryption_key, &iv, data)?;
        println!("Encrypted data: {:?}", encrypted_data);

        // Save IV and encrypted data to `encrypted_data.bin`
        let mut file =
            File::create(".config/ssh-utils/encrypted_data.bin").context("Failed to create encrypted_data.bin")?;
        file.write_all(&iv).context("Failed to write IV to file")?;
        file.write_all(&encrypted_data)
            .context("Failed to write encrypted data to file")?;
        println!("Encrypted data saved to `encrypted_data.bin`");

        // Read the encrypted data from 'encrypted_data.bin'
        let mut file =
            File::open(".config/ssh-utils/encrypted_data.bin").context("Failed to open encrypted_data.bin")?;
        let mut iv = [0u8; 16];
        file.read_exact(&mut iv)
            .context("Failed to read IV from file")?;
        let mut encrypted_data = Vec::new();
        file.read_to_end(&mut encrypted_data)
            .context("Failed to read encrypted data from file")?;

        println!("Read IV: {:?}", iv);
        println!("Read Encrypted Data: {:?}", encrypted_data);

        // Decrypt the data using the derived key and IV
        let decrypted_data = aes_decrypt(&encryption_key, &iv, &encrypted_data)?;
        println!(
            "Decrypted Data: {}",
            String::from_utf8(decrypted_data.clone()).unwrap()
        );

        assert_eq!(String::from_utf8(decrypted_data).unwrap(), "hello world");

        Ok(())
    }
}
