use aes_gcm::{
    aead::{Aead, AeadCore, KeyInit, OsRng},
    Aes256Gcm, Nonce,
};
use anyhow::{Context, Result};
use base64::{engine::general_purpose, Engine};
use aes::cipher::block_padding::Pkcs7;
use aes::cipher::{BlockDecryptMut, BlockEncryptMut, KeyIvInit};
use anyhow::anyhow;
use rand::Rng;
use scrypt::{scrypt, Params};

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
pub struct RsaAes {}

impl RsaAes {
    pub fn aes256_encrypt(data: &[u8], key: &[u8; 32], iv: &[u8; 16]) -> anyhow::Result<Vec<u8>> {
        let result = cbc::Encryptor::<aes::Aes256>::new(&(*key).into(), &(*iv).into())
            .encrypt_padded_vec_mut::<Pkcs7>(data);
        Ok(result)
    }

    pub fn _aes256_decrypt(data: &[u8], key: &[u8; 32], iv: &[u8; 16]) -> anyhow::Result<Vec<u8>> {
        let result = cbc::Decryptor::<aes::Aes256>::new(&(*key).into(), &(*iv).into())
            .decrypt_padded_vec_mut::<Pkcs7>(data)?;
        Ok(result)
    }
}

pub fn decrypt(encryption_key: &str, encrypted_text: &str) -> anyhow::Result<String> {
    let encryption_key_bytes = encryption_key.as_bytes();
    let encrypted_bytes = general_purpose::STANDARD.decode(encrypted_text)?;

    if encrypted_bytes.len() < 32 {
        return Err(anyhow!("Invalid encrypted data length"));
    }

    let salt: &[u8; 16] = &encrypted_bytes[0..16].try_into()?;
    let iv: &[u8; 16] = &encrypted_bytes[encrypted_bytes.len() - 16..].try_into()?;
    let encrypted_data = &encrypted_bytes[16..encrypted_bytes.len() - 16];

    let params = Params::new(14, 8, 1, 32)
        .map_err(|err| anyhow!("[decrypt] err={:?}", err))?;

    let mut key = [0u8; 32];
    scrypt(encryption_key_bytes, salt, &params, &mut key)
        .map_err(|err| anyhow!("[decrypt] err={:?}", err))?;

    let decipher_bytes = RsaAes::_aes256_decrypt(encrypted_data, &key, iv)?;
    let decipher_string = String::from_utf8(decipher_bytes)?;
    Ok(decipher_string)
}


pub fn encrypt(encryption_key: &str, plain_text: &str) -> anyhow::Result<String> {
    let encryption_key_bytes = encryption_key.as_bytes();
    let mut salt = [0u8; 16];
    rand::rng().fill(&mut salt);

    let params = Params::new(14, 8, 1, 32)
        .map_err(|err| anyhow!("[encrypt] err={:?}", err))?;

    let mut key = [0u8; 32];
    scrypt(encryption_key_bytes, &salt, &params, &mut key)
        .map_err(|err| anyhow!("[encrypt] err={:?}", err))?;

    let mut iv = [0u8; 16];
    rand::rng().fill(&mut iv);
    let encrypted_data = plain_text.as_bytes();
    let cipher_bytes = RsaAes::aes256_encrypt(encrypted_data, &key, &iv)?;

    let mut result = Vec::new();
    result.extend_from_slice(&salt);
    result.extend_from_slice(&cipher_bytes);
    result.extend_from_slice(&iv);

    let cipher_base64_string = general_purpose::STANDARD.encode(&result);
    Ok(cipher_base64_string)
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

