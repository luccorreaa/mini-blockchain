//! Transaction primitives for the mini-blockchain.

use ed25519_dalek::{SigningKey, Signer, VerifyingKey, Signature, Verifier};
use rand::random;
use serde::{Serialize, Deserialize};
use crate::types::PublicKey;
use crate::error::TransactionError;

/// A transfer of `amount` units from `sender` to `receiver`.
///
/// The `nonce` is randomised at construction to prevent replay attacks.
/// Call [`Transaction::sign`] before submitting to the mempool.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    sender:    PublicKey,
    receiver:  PublicKey,
    amount:    u64,
    nonce:     u64,
    signature: Option<Vec<u8>>,
}

impl Transaction {
    pub fn new(sender: PublicKey, receiver: PublicKey, amount: u64) -> Self {
        Self { sender, receiver, amount, nonce: random::<u64>(), signature: None }
    }

    /// Constructs a transaction from pre-signed components (e.g., from an API payload).
    pub fn from_parts(
        sender:    PublicKey,
        receiver:  PublicKey,
        amount:    u64,
        nonce:     u64,
        signature: Vec<u8>,
    ) -> Self {
        Self { sender, receiver, amount, nonce, signature: Some(signature) }
    }

    pub fn sign(&mut self, signing_key: &SigningKey) {
        self.signature = Some(signing_key.sign(self.signable_content().as_bytes()).to_bytes().to_vec());
    }

    /// Verifies the Ed25519 signature against this transaction's content.
    pub fn verify_signature(&self) -> Result<(), TransactionError> {
        let sig_bytes = self.signature().ok_or(TransactionError::InvalidSignature)?;
        let sig_array: [u8; 64] = sig_bytes.try_into()
            .map_err(|_| TransactionError::InvalidSignatureLength)?;
        let signature = Signature::from_bytes(&sig_array);
        let verifying_key = VerifyingKey::from_bytes(self.sender.as_bytes())
            .map_err(|_| TransactionError::InvalidPublicKey)?;
        verifying_key.verify(self.signable_content().as_bytes(), &signature)
            .map_err(|_| TransactionError::InvalidSignature)
    }

    fn signable_content(&self) -> String {
        format!(
            "{}{}{}{}",
            hex::encode(self.sender.as_bytes()),
            hex::encode(self.receiver.as_bytes()),
            self.amount,
            self.nonce,
        )
    }

    pub fn sender(&self)    -> PublicKey        { self.sender }
    pub fn receiver(&self)  -> PublicKey        { self.receiver }
    pub fn amount(&self)    -> u64              { self.amount }
    pub fn nonce(&self)     -> u64              { self.nonce }
    pub fn signature(&self) -> Option<&[u8]>   { self.signature.as_deref() }
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

    #[test]
    fn from_parts_roundtrip_verifies() {
        let signing_key = SigningKey::from_bytes(&[42u8; 32]);
        let sender = PublicKey::from_bytes(signing_key.verifying_key().to_bytes());
        let receiver = PublicKey([1u8; 32]);
        let mut tx = Transaction::new(sender, receiver, 50);
        tx.sign(&signing_key);
        let sig = tx.signature().unwrap().to_vec();
        let nonce = tx.nonce();
        let tx2 = Transaction::from_parts(sender, receiver, 50, nonce, sig);
        assert!(tx2.verify_signature().is_ok());
    }

    #[test]
    fn verify_signature_fails_on_bad_signature() {
        let sender = PublicKey([1u8; 32]);
        let receiver = PublicKey([2u8; 32]);
        let tx = Transaction::from_parts(sender, receiver, 50, 12345, vec![0u8; 64]);
        assert!(tx.verify_signature().is_err());
    }

    #[test]
    fn verify_signature_fails_when_unsigned() {
        let tx = Transaction::new(PublicKey::coinbase(), PublicKey([1u8; 32]), 10);
        assert!(tx.verify_signature().is_err());
    }
}
