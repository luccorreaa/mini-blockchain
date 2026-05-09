//transactions.rs
use ed25519_dalek::SigningKey;
use ed25519_dalek::Signer;
use hex;
use serde::{Serialize, Deserialize};

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Transaction {
    #[serde(with = "hex")]
    pub sender: [u8; 32],
    #[serde(with = "hex")]
    pub receiver: [u8; 32],
    pub amount: u64,
    pub firma: Option<Vec<u8>>,
}



impl Transaction {
    pub fn new(sender: [u8; 32], receiver: [u8; 32], amount: u64) -> Transaction {
        Transaction {
            sender,
            receiver,
            amount,
            firma: None
        }
    }
    pub fn firmar(&mut self, signing_key: &SigningKey) {
        let contenido = format!("{}{}{}", hex::encode(self.sender), hex::encode(self.receiver), self.amount);

        let signature = signing_key.sign(contenido.as_bytes());
        self.firma = Some(signature.to_bytes().to_vec());
    }

}