use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};

const NONCE_SIZE: usize = 12; // 96 bits for AES-GCM

pub fn encrypt_private_key(private_key: &str, encryption_key: &str) -> Result<String> {
    let key_bytes = if encryption_key.len() >= 32 {
        encryption_key.as_bytes()[..32].to_vec()
    } else {
        let mut key = encryption_key.as_bytes().to_vec();
        key.resize(32, 0);
        key
    };

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .context("Failed to create cipher from key")?;
    
    let nonce = Aes256Gcm::generate_nonce(&mut OsRng);
    
    let ciphertext = cipher
        .encrypt(&nonce, private_key.as_bytes())
        .map_err(|_| anyhow::anyhow!("Failed to encrypt private key"))?;
    
    let mut encrypted_data = nonce.to_vec();
    encrypted_data.extend_from_slice(&ciphertext);
    
    Ok(general_purpose::STANDARD.encode(&encrypted_data))
}

pub fn decrypt_private_key(encrypted_data: &str, encryption_key: &str) -> Result<String> {
    let key_bytes = if encryption_key.len() >= 32 {
        encryption_key.as_bytes()[..32].to_vec()
    } else {
        let mut key = encryption_key.as_bytes().to_vec();
        key.resize(32, 0);
        key
    };

    let cipher = Aes256Gcm::new_from_slice(&key_bytes)
        .context("Failed to create cipher from key")?;
    
    let encrypted_bytes = general_purpose::STANDARD
        .decode(encrypted_data)
        .context("Failed to decode base64 encrypted data")?;
    
    if encrypted_bytes.len() < NONCE_SIZE {
        anyhow::bail!("Encrypted data too short");
    }

    let nonce = Nonce::from_slice(&encrypted_bytes[..NONCE_SIZE]);
    let ciphertext = &encrypted_bytes[NONCE_SIZE..];
    
    let plaintext = cipher
        .decrypt(nonce, ciphertext)
        .map_err(|_| anyhow::anyhow!("Failed to decrypt private key"))?;

    String::from_utf8(plaintext).context("Failed to convert decrypted data to string")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_encrypt_decrypt() {
        let encryption_key = "my-secret-encryption-key-32-bytes-long!!";
        let private_key = "0x1234567890abcdef1234567890abcdef1234567890abcdef1234567890abcdef";

        let encrypted = encrypt_private_key(private_key, encryption_key).unwrap();
        assert_ne!(encrypted, private_key);

        let decrypted = decrypt_private_key(&encrypted, encryption_key).unwrap();
        assert_eq!(decrypted, private_key);
    }
}

