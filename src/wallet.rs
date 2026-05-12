use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::OsRng;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};

#[derive(Serialize, Deserialize)]
pub struct Wallet {
    #[serde(with = "hex")]
    pub secret: [u8; 32],
    #[serde(with = "hex")]
    pub pubkey: [u8; 32],
}

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
    pub fn new(secret: [u8; 32], pubkey: [u8; 32]) -> Wallet {
        Wallet { secret, pubkey }
    }

    fn derive_key(password: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.finalize().into()
    }

    pub fn guardar_cifrado(&self, path: &str, password: &str) -> Result<(), Box<dyn std::error::Error>> {
        let key_bytes = Self::derive_key(password);
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);

        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);

        let ciphertext = cipher.encrypt(nonce, self.secret.as_ref())
            .map_err(|e| format!("Error al cifrar: {}", e))?;

        let encrypted = EncryptedWallet { nonce: nonce_bytes, ciphertext, pubkey: self.pubkey };
        let json = serde_json::to_string_pretty(&encrypted)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn cargar_cifrado(path: &str, password: &str) -> Result<Wallet, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let encrypted: EncryptedWallet = serde_json::from_str(&json)?;

        let key_bytes = Self::derive_key(password);
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&encrypted.nonce);

        let plaintext = cipher.decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|_| "Contraseña incorrecta o archivo corrupto")?;

        let secret: [u8; 32] = plaintext.try_into()
            .map_err(|_| "Longitud de clave incorrecta")?;

        Ok(Wallet { secret, pubkey: encrypted.pubkey })
    }

    // Métodos legacy sin cifrado (compatibilidad con tests existentes)
    pub fn guardar(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn cargar(path: &str) -> Result<Wallet, Box<dyn std::error::Error>> {
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
        wallet.guardar_cifrado("/tmp/test_wallet_rt.json", "mi_password").unwrap();
        let loaded = Wallet::cargar_cifrado("/tmp/test_wallet_rt.json", "mi_password").unwrap();
        assert_eq!(loaded.secret, wallet.secret);
        assert_eq!(loaded.pubkey, wallet.pubkey);
    }

    #[test]
    fn password_incorrecta_falla() {
        let wallet = Wallet::new([1u8; 32], [2u8; 32]);
        wallet.guardar_cifrado("/tmp/test_wallet_pw.json", "correcta").unwrap();
        assert!(Wallet::cargar_cifrado("/tmp/test_wallet_pw.json", "incorrecta").is_err());
    }
}
