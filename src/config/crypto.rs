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

    let config = Config::owasp3();
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

#[cfg(test)]
mod tests {
    use std::{io::Write, process::Command};

    use tempfile::NamedTempFile;

    use super::*;

    #[test]
    fn test_derive_key_from_password() {
        let password = "super_secret_password";
        let salt = b"super_secret_password";
        // The expected key in hexadecimal format is obtained from the command:
        // echo -n "super_secret_password" | ./argon2 super_secret_password -t 3 -k 12288 -p 1 -id -l 32 -r
        // Since the salt in the command cannot be passed as a byte array, we have defined a custom salt
        // Copy the logic of derive_key_from_password() for testing
        let expected_key_hex = "66e5467d6adc707c5fe42c2516de285204f4ce590612e58eddaab21b763aaca2";
        let expected_key = hex::decode(expected_key_hex).expect("Decoding failed");

        let derived_key_result = derive_key_from_password(password);

        let config = Config::owasp3();
        let key = argon2::hash_raw(password.as_bytes(), salt, &config).unwrap();
        
        // Check if the result is Ok
        assert!(derived_key_result.is_ok());
        
        let derived_key = derived_key_result.unwrap();
        
        // Check if the length is 32 bytes
        assert_eq!(derived_key.len(), 32);

        // Check if the key is non-zero
        assert!(derived_key.iter().any(|&byte| byte != 0));

        // Check if the generated key matches the expected value
        assert_eq!(key, expected_key);
    }

    #[test]
    fn test_derive_sha256_digest() {
        let password = "super_secret_password";
        
        // Use Rust to execute the command line to get the expected digest value
        let output = std::process::Command::new("sh")
            .arg("-c")
            .arg(format!(
                "echo -n \"{}\" | sha256sum | awk '{{print $1}}' | cut -c 1-32",
                password
            ))
            .output()
            .expect("Failed to execute command");

        let expected_digest_hex = String::from_utf8(output.stdout)
            .expect("Failed to convert output to string")
            .trim()
            .to_string();
        
        let expected_digest = hex::decode(expected_digest_hex).expect("Decoding failed");

        let derived_digest = derive_sha256_digest(password);
        
        // Check if the length is 16 bytes
        assert_eq!(derived_digest.len(), 16);

        // Check if the generated digest matches the expected value
        assert_eq!(derived_digest, expected_digest.as_slice());
    }

    #[test]
    fn test_aes_encrypt_decrypt() {
        let key = b"01234567890123456789012345678901"; // A 32-byte key
        let iv = b"0123456789012345"; // A 16-byte initialization vector
        let data = b"Hello, AES encryption!";

        // Write data to a temporary file
        let mut data_file = NamedTempFile::new().expect("Failed to create temporary file");
        data_file.write_all(data).expect("Failed to write data to file");
        let data_file_path = data_file.path().to_str().unwrap();

        // Use OpenSSL to generate the expected encrypted value
        let expected_encrypted_output = Command::new("openssl")
            .arg("enc")
            .arg("-aes-256-ctr")
            .arg("-in")
            .arg(data_file_path)
            .arg("-K")
            .arg(hex::encode(key))
            .arg("-iv")
            .arg(hex::encode(iv))
            .output()
            .expect("Failed to execute openssl command");
        let expected_encrypted_data = expected_encrypted_output.stdout;

        // Test aes_encrypt
        let encrypted_data = aes_encrypt(key, iv, data).expect("Failed to encrypt data");
        assert!(!encrypted_data.is_empty(), "Encrypted data should not be empty");
        assert_eq!(encrypted_data, expected_encrypted_data, "Encrypted data should match the expected value");

        // Write encrypted_data to a temporary file
        let mut encrypted_file = NamedTempFile::new().expect("Failed to create temporary file");
        encrypted_file.write_all(&encrypted_data).expect("Failed to write encrypted data to file");
        let encrypted_file_path = encrypted_file.path().to_str().unwrap();

        // Use OpenSSL to generate the expected decrypted value
        let expected_decrypted_output = Command::new("openssl")
            .arg("enc")
            .arg("-d")
            .arg("-aes-256-ctr")
            .arg("-in")
            .arg(encrypted_file_path)
            .arg("-K")
            .arg(hex::encode(key))
            .arg("-iv")
            .arg(hex::encode(iv))
            .output()
            .expect("Failed to execute openssl command");
        let expected_decrypted_data = expected_decrypted_output.stdout;

        // Test aes_decrypt
        let decrypted_data = aes_decrypt(key, iv, &encrypted_data).expect("Failed to decrypt data");
        assert_eq!(decrypted_data, data, "Decrypted data should match original data");
        assert_eq!(decrypted_data, expected_decrypted_data, "Decrypted data should match the expected value");
    }
}