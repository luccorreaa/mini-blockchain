//! Wallet: Ed25519 key pair with BIP-39 mnemonic generation and AES-256-GCM encrypted persistence.

use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::OsRng;
use bip39::Mnemonic;
use ed25519_dalek::SigningKey;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use crate::types::PublicKey;
use crate::error::{WalletError, WalletResult};

/// An Ed25519 key pair used for signing transactions and blocks.
#[derive(Serialize, Deserialize)]
pub struct Wallet {
    #[serde(with = "hex")]
    secret: [u8; 32],
    pubkey: PublicKey,
}

#[derive(Serialize, Deserialize)]
struct EncryptedWallet {
    #[serde(with = "hex")]
    nonce: [u8; 12],
    #[serde(with = "hex")]
    ciphertext: Vec<u8>,
    pubkey: PublicKey,
}

/// Derives an Ed25519 signing key from a BIP-39 mnemonic phrase.
///
/// Uses the first 32 bytes of the 64-byte BIP-39 seed (no HD derivation).
pub fn signing_key_from_mnemonic(phrase: &str) -> WalletResult<SigningKey> {
    let mnemonic = Mnemonic::parse(phrase)
        .map_err(|e| WalletError::InvalidMnemonic(e.to_string()))?;
    let seed = mnemonic.to_seed("");
    let mut secret = [0u8; 32];
    secret.copy_from_slice(&seed[..32]);
    Ok(SigningKey::from_bytes(&secret))
}

impl Wallet {
    pub fn new(secret: [u8; 32], pubkey: PublicKey) -> Self {
        Self { secret, pubkey }
    }

    /// Generates a new wallet from a fresh BIP-39 mnemonic.
    ///
    /// Returns the mnemonic (show to user, never store) and the wallet.
    pub fn generate() -> WalletResult<(Mnemonic, Self)> {
        let mnemonic = Mnemonic::generate(12)
            .map_err(|e| WalletError::InvalidMnemonic(e.to_string()))?;
        let seed = mnemonic.to_seed("");
        let mut secret = [0u8; 32];
        secret.copy_from_slice(&seed[..32]);
        let signing_key = SigningKey::from_bytes(&secret);
        let pubkey = PublicKey::from_bytes(signing_key.verifying_key().to_bytes());
        Ok((mnemonic, Self { secret, pubkey }))
    }

    /// Reconstructs a wallet from an existing BIP-39 mnemonic phrase.
    pub fn from_mnemonic(phrase: &str) -> WalletResult<Self> {
        let signing_key = signing_key_from_mnemonic(phrase)?;
        let pubkey = PublicKey::from_bytes(signing_key.verifying_key().to_bytes());
        Ok(Self { secret: signing_key.to_bytes(), pubkey })
    }

    pub fn secret(&self) -> &[u8; 32] { &self.secret }
    pub fn pubkey(&self) -> PublicKey  { self.pubkey }

    fn derive_key(password: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.finalize().into()
    }

    pub fn save_encrypted(&self, path: impl AsRef<std::path::Path>, password: &str) -> WalletResult<()> {
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

    pub fn load_encrypted(path: impl AsRef<std::path::Path>, password: &str) -> WalletResult<Wallet> {
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

    pub fn save(&self, path: impl AsRef<std::path::Path>) -> WalletResult<()> {
        std::fs::write(path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }

    pub fn load(path: impl AsRef<std::path::Path>) -> WalletResult<Wallet> {
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
        assert_eq!(loaded.secret(), wallet.secret());
        assert_eq!(loaded.pubkey(), wallet.pubkey());
    }

    #[test]
    fn password_incorrecta_falla() {
        let wallet = Wallet::new([1u8; 32], PublicKey([2u8; 32]));
        wallet.save_encrypted("/tmp/test_wallet_pw.json", "correcta").unwrap();
        assert!(Wallet::load_encrypted("/tmp/test_wallet_pw.json", "incorrecta").is_err());
    }

    #[test]
    fn generate_produces_12_word_mnemonic() {
        let (mnemonic, wallet) = Wallet::generate().unwrap();
        let phrase = mnemonic.to_string();
        assert_eq!(phrase.split_whitespace().count(), 12);
        let _ = wallet.pubkey();
    }

    #[test]
    fn from_mnemonic_is_deterministic() {
        let phrase = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
        let w1 = Wallet::from_mnemonic(phrase).unwrap();
        let w2 = Wallet::from_mnemonic(phrase).unwrap();
        assert_eq!(w1.pubkey(), w2.pubkey());
    }

    #[test]
    fn generate_and_from_mnemonic_produce_same_wallet() {
        let (mnemonic, wallet) = Wallet::generate().unwrap();
        let restored = Wallet::from_mnemonic(&mnemonic.to_string()).unwrap();
        assert_eq!(restored.pubkey(), wallet.pubkey());
        assert_eq!(restored.secret(), wallet.secret());
    }

    #[test]
    fn invalid_mnemonic_returns_error() {
        assert!(Wallet::from_mnemonic("not a valid phrase at all").is_err());
        assert!(signing_key_from_mnemonic("bad input").is_err());
    }
}
