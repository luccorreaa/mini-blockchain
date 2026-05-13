//! Transaction primitives for the mini-blockchain.
//!
//! A [`Transaction`] represents a transfer of value between two Ed25519 public keys.
//! Each transaction carries a random nonce to prevent replay attacks and an optional
//! Ed25519 signature produced by the sender's signing key.

use ed25519_dalek::SigningKey;
use ed25519_dalek::Signer;
use hex;
use rand::random;
use serde::{Serialize, Deserialize};

/// A transfer of `amount` units from `sender` to `receiver`.
///
/// The `nonce` field is randomised at construction time to prevent replay attacks.
/// Call [`Transaction::sign`] before adding the transaction to the mempool; unsigned
/// non-coinbase transactions are rejected during chain validation.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    /// Sender's Ed25519 public key (32 bytes, hex-encoded in JSON).
    /// All-zero bytes indicate a coinbase transaction (no sender).
    #[serde(with = "hex")]
    pub sender: [u8; 32],
    /// Recipient's Ed25519 public key (32 bytes, hex-encoded in JSON).
    #[serde(with = "hex")]
    pub receiver: [u8; 32],
    /// Amount of units to transfer.
    pub amount: u64,
    /// Random nonce assigned at construction to prevent replay attacks.
    pub nonce: u64,
    /// Ed25519 signature over `hex(sender) || hex(receiver) || amount || nonce`.
    /// `None` for coinbase transactions; required for all others.
    pub signature: Option<Vec<u8>>,
}

impl Transaction {
    /// Creates a new unsigned transaction.
    ///
    /// A cryptographically random `nonce` is assigned automatically.
    /// Pass all-zero bytes as `sender` to create a coinbase transaction.
    pub fn new(sender: [u8; 32], receiver: [u8; 32], amount: u64) -> Transaction {
        Transaction {
            sender,
            receiver,
            amount,
            nonce: random::<u64>(),
            signature: None,
        }
    }

    /// Signs the transaction with the given Ed25519 signing key.
    ///
    /// The signed message is `hex(sender) || hex(receiver) || amount || nonce`.
    /// Must be called before submitting the transaction to the mempool.
    pub fn sign(&mut self, signing_key: &SigningKey) {
        let content = format!(
            "{}{}{}{}",
            hex::encode(self.sender),
            hex::encode(self.receiver),
            self.amount,
            self.nonce
        );
        let signature = signing_key.sign(content.as_bytes());
        self.signature = Some(signature.to_bytes().to_vec());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transacciones_identicas_tienen_nonce_distinto() {
        let tx1 = Transaction::new([0u8; 32], [1u8; 32], 100);
        let tx2 = Transaction::new([0u8; 32], [1u8; 32], 100);
        assert_ne!(tx1.nonce, tx2.nonce);
    }
}
