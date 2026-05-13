//! Block structure and Proof-of-Work mining.
//!
//! A [`Block`] is the fundamental unit of the chain. It commits to a set of
//! transactions via their Merkle root and links to its predecessor through
//! `prev_hash`. Mining increments the `nonce` until the block's SHA-256 hash
//! has the required number of leading zero hex digits.

use ed25519_dalek::SigningKey;
use ed25519_dalek::Signer;
use sha2::{Sha256, Digest};
use std::time::SystemTime;
use crate::transactions::Transaction;
use serde::{Serialize, Deserialize};
use crate::merkle::merkle_root;

/// A single block in the blockchain.
///
/// Each block commits to its position (`index`), the previous block's hash
/// (`prev_hash`), a set of transactions (via Merkle root), a Unix timestamp,
/// and a proof-of-work `nonce`. An optional Ed25519 signature and `author`
/// public key allow attributing block production to a specific node.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    index: u32,
    transactions: Vec<Transaction>,
    prev_hash: String,
    hash: String,
    timestamp: u64,
    signature: Option<Vec<u8>>,
    author: Option<[u8; 32]>,
    nonce: u64,
}

impl Block {
    /// Computes the SHA-256 hash of this block's header fields.
    ///
    /// Covers `index`, Merkle root of `transactions`, `prev_hash`, `timestamp`, and `nonce`.
    pub fn compute_hash(&self) -> String {
        let mut hasher = Sha256::new();
        let content = format!(
            "{}{}{}{}{}",
            self.index, merkle_root(&self.transactions), self.prev_hash, self.timestamp, self.nonce
        );
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    /// Creates a new unmined block at `index` with the given `transactions` and `prev_hash`.
    ///
    /// The hash is computed immediately with `nonce = 0`. Call [`Block::mine`] before
    /// appending to the chain.
    ///
    /// # Panics
    ///
    /// Panics if the system clock is set to before the Unix epoch.
    pub fn new(index: u32, transactions: Vec<Transaction>, prev_hash: &str) -> Block {
        let mut block = Block {
            index,
            transactions,
            prev_hash: String::from(prev_hash),
            hash: String::new(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            signature: None,
            author: None,
            nonce: 0,
        };
        block.hash = block.compute_hash();
        block
    }

    /// Signs the block with the given Ed25519 signing key.
    ///
    /// The signed message is `index || merkle_root || prev_hash || timestamp`.
    /// Sets both `signature` and `author` fields.
    pub fn sign(&mut self, signing_key: &SigningKey) {
        let content = format!(
            "{}{}{}{}",
            self.index, merkle_root(&self.transactions), self.prev_hash, self.timestamp
        );
        let signature = signing_key.sign(content.as_bytes());
        self.signature = Some(signature.to_bytes().to_vec());
        self.author = Some(signing_key.verifying_key().to_bytes());
    }

    /// Performs Proof-of-Work mining until `hash` starts with `difficulty` leading zero hex digits.
    ///
    /// Increments `nonce` and recomputes the hash on each iteration.
    pub fn mine(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);
        while !self.hash.starts_with(&target) {
            self.nonce += 1;
            self.hash = self.compute_hash();
        }
    }

    /// Returns the block's SHA-256 hash.
    pub fn hash(&self) -> &str { &self.hash }
    /// Returns the hash of the preceding block.
    pub fn prev_hash(&self) -> &str { &self.prev_hash }
    /// Returns the block's position in the chain (0-based).
    pub fn index(&self) -> u32 { self.index }
    /// Returns the Unix timestamp (seconds) recorded at block creation.
    pub fn timestamp(&self) -> u64 { self.timestamp }
    /// Returns the transactions included in this block.
    pub fn transactions(&self) -> &[Transaction] { &self.transactions }
    /// Returns the block's Ed25519 signature, if present.
    pub fn signature(&self) -> &Option<Vec<u8>> { &self.signature }
    /// Returns the author's Ed25519 public key, if the block was signed.
    pub fn author(&self) -> Option<[u8; 32]> { self.author }

    #[cfg(test)]
    pub fn corrupt(&mut self) {
        self.hash = "corrupt_hash".to_string();
    }

    #[cfg(test)]
    pub fn set_signature_test(&mut self, sig: Vec<u8>) {
        self.signature = Some(sig);
    }

    #[cfg(test)]
    pub fn set_author_test(&mut self, author: [u8; 32]) {
        self.author = Some(author);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_es_consistente_al_recalcular() {
        let block = Block::new(0, vec![], "0");
        assert_eq!(block.hash(), block.compute_hash());
    }

    #[test]
    fn hash_cambia_al_agregar_transaccion() {
        let mut block = Block::new(0, vec![], "0");
        let hash_original = block.compute_hash();
        block.transactions.push(Transaction::new([0u8; 32], [1u8; 32], 100));
        assert_ne!(hash_original, block.compute_hash());
    }
}
