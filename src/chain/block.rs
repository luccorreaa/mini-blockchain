//! Block structure and Proof-of-Work mining.

use ed25519_dalek::{SigningKey, Signer};
use sha2::{Sha256, Digest};
use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use crate::crypto::transaction::Transaction;
use crate::chain::merkle::merkle_root;
use crate::types::{Hash, PublicKey};

/// A single block in the blockchain.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    index: u32,
    transactions: Vec<Transaction>,
    prev_hash: Hash,
    hash: Hash,
    timestamp: u64,
    signature: Option<Vec<u8>>,
    author: Option<PublicKey>,
    nonce: u64,
}

impl Block {
    /// Computes the SHA-256 hash of this block's header fields.
    pub fn compute_hash(&self) -> Hash {
        let content = format!(
            "{}{}{}{}{}",
            self.index, merkle_root(&self.transactions), self.prev_hash, self.timestamp, self.nonce
        );
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        Hash::from(format!("{:x}", hasher.finalize()))
    }

    /// Creates a new unmined block. Call [`Block::mine`] before appending to the chain.
    pub fn new(index: u32, transactions: Vec<Transaction>, prev_hash: &Hash) -> Block {
        let mut block = Block {
            index,
            transactions,
            prev_hash: prev_hash.clone(),
            hash: Hash::empty(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .expect("system clock before UNIX_EPOCH")
                .as_secs(),
            signature: None,
            author: None,
            nonce: 0,
        };
        block.hash = block.compute_hash();
        block
    }

    /// Signs the block with the given Ed25519 signing key.
    pub fn sign(&mut self, signing_key: &SigningKey) {
        let content = format!(
            "{}{}{}{}",
            self.index, merkle_root(&self.transactions), self.prev_hash, self.timestamp
        );
        let signature = signing_key.sign(content.as_bytes());
        self.signature = Some(signature.to_bytes().to_vec());
        self.author = Some(PublicKey::from_bytes(signing_key.verifying_key().to_bytes()));
    }

    /// Increments nonce until hash has `difficulty` leading zero hex digits.
    pub fn mine(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);
        while !self.hash.as_str().starts_with(&target) {
            self.nonce += 1;
            self.hash = self.compute_hash();
        }
    }

    pub fn hash(&self) -> &Hash { &self.hash }
    pub fn prev_hash(&self) -> &Hash { &self.prev_hash }
    pub fn index(&self) -> u32 { self.index }
    pub fn timestamp(&self) -> u64 { self.timestamp }
    pub fn transactions(&self) -> &[Transaction] { &self.transactions }
    pub fn signature(&self) -> Option<&[u8]> { self.signature.as_deref() }
    pub fn author(&self) -> Option<PublicKey> { self.author }

    #[cfg(test)]
    pub fn corrupt(&mut self) { self.hash = Hash::from("corrupt_hash".to_string()); }

    #[cfg(test)]
    pub fn set_signature_test(&mut self, sig: Vec<u8>) { self.signature = Some(sig); }

    #[cfg(test)]
    pub fn set_author_test(&mut self, author: PublicKey) { self.author = Some(author); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_es_consistente_al_recalcular() {
        let block = Block::new(0, vec![], &Hash::empty());
        assert_eq!(block.hash(), &block.compute_hash());
    }

    #[test]
    fn hash_cambia_al_agregar_transaccion() {
        let mut block = Block::new(0, vec![], &Hash::empty());
        let hash_original = block.compute_hash();
        block.transactions.push(
            Transaction::new(PublicKey::coinbase(), PublicKey([1u8; 32]), 100)
        );
        assert_ne!(hash_original, block.compute_hash());
    }
}
