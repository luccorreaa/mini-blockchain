//wallet.rs
use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize)]
pub struct Wallet {
    #[serde(with = "hex")]
    pub secret: [u8; 32],  // hex del secret
    #[serde(with = "hex")]
    pub pubkey: [u8; 32],  // hex de la clave pública
}

impl Wallet {
    pub fn guardar(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    pub fn cargar(path: &str) -> Result<Wallet, Box<dyn std::error::Error>>{
        let json = std::fs::read_to_string(path)?;
        let wallet = serde_json::from_str(&json)?;
        Ok(wallet)
    }
    pub fn new(secret: [u8; 32], pubkey: [u8; 32]) -> Wallet {
        Wallet {
            secret,
            pubkey,
        }
        
    }
 }   