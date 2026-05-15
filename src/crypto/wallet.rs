//! Wallet: Ed25519 key pair with AES-256-GCM encrypted persistence.

use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::OsRng;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use crate::types::PublicKey;
use crate::error::{WalletError, WalletResult};

/// An Ed25519 key pair used for signing transactions and blocks.
#[derive(Serialize, Deserialize)]
pub struct Wallet {
    /// Raw 32-byte Ed25519 secret (signing) key, hex-encoded in JSON.
    #[serde(with = "hex")]
    pub secret: [u8; 32],
    /// Corresponding Ed25519 public (verifying) key.
    pub pubkey: PublicKey,
}

#[derive(Serialize, Deserialize)]
struct EncryptedWallet {
    #[serde(with = "hex")]
    nonce: [u8; 12],
    #[serde(with = "hex")]
    ciphertext: Vec<u8>,
    pubkey: PublicKey,
}

impl Wallet {
    pub fn new(secret: [u8; 32], pubkey: PublicKey) -> Self {
        Self { secret, pubkey }
    }

    fn derive_key(password: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.finalize().into()
    }

    pub fn save_encrypted(&self, path: &str, password: &str) -> WalletResult<()> {
        let key_bytes = Self::derive_key(password);
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, self.secret.as_ref())
            .map_err(|e| WalletError::EncryptionFailed(e.to_string()))?;
        let encrypted = EncryptedWallet { nonce: nonce_bytes, ciphertext, pubkey: self.pubkey };
        std::fs::write(path, serde_json::to_string_pretty(&encrypted)?)?;
        Ok(())
    }

    pub fn load_encrypted(path: &str, password: &str) -> WalletResult<Wallet> {
        let encrypted: EncryptedWallet = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        let key_bytes = Self::derive_key(password);
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&encrypted.nonce);
        let plaintext = cipher.decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|_| WalletError::WrongPassword)?;
        let secret: [u8; 32] = plaintext.try_into()
            .map_err(|_| WalletError::DecryptedKeyLength)?;
        Ok(Wallet { secret, pubkey: encrypted.pubkey })
    }

    pub fn save(&self, path: &str) -> WalletResult<()> {
        std::fs::write(path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }

    pub fn load(path: &str) -> WalletResult<Wallet> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cifrado_round_trip() {
        let wallet = Wallet::new([42u8; 32], PublicKey([7u8; 32]));
        wallet.save_encrypted("/tmp/test_wallet_rt.json", "mi_password").unwrap();
        let loaded = Wallet::load_encrypted("/tmp/test_wallet_rt.json", "mi_password").unwrap();
        assert_eq!(loaded.secret, wallet.secret);
        assert_eq!(loaded.pubkey, wallet.pubkey);
    }

    #[test]
    fn password_incorrecta_falla() {
        let wallet = Wallet::new([1u8; 32], PublicKey([2u8; 32]));
        wallet.save_encrypted("/tmp/test_wallet_pw.json", "correcta").unwrap();
        assert!(Wallet::load_encrypted("/tmp/test_wallet_pw.json", "incorrecta").is_err());
    }
}
