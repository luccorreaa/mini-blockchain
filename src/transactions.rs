use ed25519_dalek::SigningKey;
use ed25519_dalek::Signer;
use hex;
use rand::random;
use serde::{Serialize, Deserialize};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    #[serde(with = "hex")]
    pub sender: [u8; 32],
    #[serde(with = "hex")]
    pub receiver: [u8; 32],
    pub amount: u64,
    pub nonce: u64,
    pub firma: Option<Vec<u8>>,
}

impl Transaction {
    pub fn new(sender: [u8; 32], receiver: [u8; 32], amount: u64) -> Transaction {
        Transaction {
            sender,
            receiver,
            amount,
            nonce: random::<u64>(),
            firma: None,
        }
    }

    pub fn firmar(&mut self, signing_key: &SigningKey) {
        let contenido = format!(
            "{}{}{}{}",
            hex::encode(self.sender),
            hex::encode(self.receiver),
            self.amount,
            self.nonce
        );
        let signature = signing_key.sign(contenido.as_bytes());
        self.firma = Some(signature.to_bytes().to_vec());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transacciones_identicas_tienen_nonce_distinto() {
        let tx1 = Transaction::new([0u8; 32], [1u8; 32], 100);
        let tx2 = Transaction::new([0u8; 32], [1u8; 32], 100);
        // Estadísticamente casi imposible que dos random u64 sean iguales
        assert_ne!(tx1.nonce, tx2.nonce);
    }
}
