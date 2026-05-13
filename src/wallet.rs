//! Wallet: Ed25519 key pair with AES-256-GCM encrypted persistence.
//!
//! A [`Wallet`] holds a raw Ed25519 secret key and its corresponding public key.
//! Keys are stored on disk encrypted with AES-256-GCM, where the encryption key
//! is derived from a user-supplied password via SHA-256.

use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::OsRng;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

/// An Ed25519 key pair used for signing transactions and blocks.
#[derive(Serialize, Deserialize)]
pub struct Wallet {
    /// Raw 32-byte Ed25519 secret (signing) key, hex-encoded in JSON.
    #[serde(with = "hex")]
    pub secret: [u8; 32],
    /// Corresponding Ed25519 public (verifying) key, hex-encoded in JSON.
    #[serde(with = "hex")]
    pub pubkey: [u8; 32],
}

/// On-disk representation of an AES-256-GCM encrypted wallet.
#[derive(Serialize, Deserialize)]
struct EncryptedWallet {
    #[serde(with = "hex")]
    nonce: [u8; 12],
    #[serde(with = "hex")]
    ciphertext: Vec<u8>,
    #[serde(with = "hex")]
    pubkey: [u8; 32],
}

impl Wallet {
    /// Creates a new wallet from a raw secret key and its public key.
    pub fn new(secret: [u8; 32], pubkey: [u8; 32]) -> Wallet {
        Wallet { secret, pubkey }
    }

    /// Derives a 32-byte AES key from `password` using SHA-256.
    fn derive_key(password: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.finalize().into()
    }

    /// Encrypts and saves the wallet's secret key to `path` using `password`.
    ///
    /// The public key is stored in plaintext alongside the ciphertext so the
    /// file can be identified without decrypting it.
    ///
    /// # Errors
    ///
    /// Returns an error if encryption or file I/O fails.
    pub fn save_encrypted(&self, path: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let key_bytes = Self::derive_key(password);
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, self.secret.as_ref())
            .map_err(|e| format!("Encryption error: {}", e))?;

        let encrypted = EncryptedWallet { nonce: nonce_bytes, ciphertext, pubkey: self.pubkey };
        let json = serde_json::to_string_pretty(&encrypted)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Loads and decrypts a wallet from the JSON file at `path` using `password`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read, decryption fails (wrong password),
    /// or the decrypted payload has an unexpected length.
    pub fn load_encrypted(path: &str, password: &str) -> Result<Wallet, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let encrypted: EncryptedWallet = serde_json::from_str(&json)?;

        let key_bytes = Self::derive_key(password);
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&encrypted.nonce);

        let plaintext = cipher.decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|_| "Wrong password or corrupted file")?;

        let secret: [u8; 32] = plaintext.try_into()
            .map_err(|_| "Unexpected key length in decrypted payload")?;

        Ok(Wallet { secret, pubkey: encrypted.pubkey })
    }

    /// Saves the wallet as plain JSON (no encryption) to `path`.
    ///
    /// Prefer [`Wallet::save_encrypted`] for any non-test usage.
    ///
    /// # Errors
    ///
    /// Returns an error if serialisation or file I/O fails.
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Loads a plain-JSON (unencrypted) wallet from `path`.
    ///
    /// Prefer [`Wallet::load_encrypted`] for any non-test usage.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &str) -> Result<Wallet, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let wallet = serde_json::from_str(&json)?;
        Ok(wallet)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cifrado_round_trip() {
        let wallet = Wallet::new([42u8; 32], [7u8; 32]);
        wallet.save_encrypted("/tmp/test_wallet_rt.json", "mi_password").unwrap();
        let loaded = Wallet::load_encrypted("/tmp/test_wallet_rt.json", "mi_password").unwrap();
        assert_eq!(loaded.secret, wallet.secret);
        assert_eq!(loaded.pubkey, wallet.pubkey);
    }

    #[test]
    fn password_incorrecta_falla() {
        let wallet = Wallet::new([1u8; 32], [2u8; 32]);
        wallet.save_encrypted("/tmp/test_wallet_pw.json", "correcta").unwrap();
        assert!(Wallet::load_encrypted("/tmp/test_wallet_pw.json", "incorrecta").is_err());
    }
}
