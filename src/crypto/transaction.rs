//! Transaction primitives for the mini-blockchain.

use ed25519_dalek::{SigningKey, Signer};
use rand::random;
use serde::{Serialize, Deserialize};
use crate::types::PublicKey;

/// A transfer of `amount` units from `sender` to `receiver`.
///
/// The `nonce` is randomised at construction to prevent replay attacks.
/// Call [`Transaction::sign`] before submitting to the mempool.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    sender: PublicKey,
    receiver: PublicKey,
    amount: u64,
    nonce: u64,
    signature: Option<Vec<u8>>,
}

impl Transaction {
    pub fn new(sender: PublicKey, receiver: PublicKey, amount: u64) -> Self {
        Self {
            sender,
            receiver,
            amount,
            nonce: random::<u64>(),
            signature: None,
        }
    }

    pub fn sign(&mut self, signing_key: &SigningKey) {
        let content = format!(
            "{}{}{}{}",
            hex::encode(self.sender.as_bytes()),
            hex::encode(self.receiver.as_bytes()),
            self.amount,
            self.nonce
        );
        self.signature = Some(signing_key.sign(content.as_bytes()).to_bytes().to_vec());
    }

    pub fn sender(&self) -> PublicKey { self.sender }
    pub fn receiver(&self) -> PublicKey { self.receiver }
    pub fn amount(&self) -> u64 { self.amount }
    pub fn nonce(&self) -> u64 { self.nonce }
    pub fn signature(&self) -> Option<&[u8]> { self.signature.as_deref() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transacciones_identicas_tienen_nonce_distinto() {
        let tx1 = Transaction::new(PublicKey::coinbase(), PublicKey([1u8; 32]), 100);
        let tx2 = Transaction::new(PublicKey::coinbase(), PublicKey([1u8; 32]), 100);
        assert_ne!(tx1.nonce(), tx2.nonce());
    }
}
