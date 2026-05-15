//! Merkle tree root computation for transaction sets.

use sha2::{Sha256, Digest};
use crate::crypto::transaction::Transaction;
use crate::types::Hash;

fn sha256_hex(content: &str) -> Hash {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    Hash::from(format!("{:x}", h.finalize()))
}

/// Computes the SHA-256 Merkle root of `transactions`.
///
/// Returns an empty Hash when `transactions` is empty.
/// Odd-length layers duplicate the last hash before combining pairs.
pub fn merkle_root(transactions: &[Transaction]) -> Hash {
    if transactions.is_empty() {
        return Hash::empty();
    }
    let mut hashes: Vec<Hash> = transactions.iter().map(|tx| {
        sha256_hex(&format!(
            "{}{}{}{}",
            hex::encode(tx.sender().as_bytes()),
            hex::encode(tx.receiver().as_bytes()),
            tx.amount(),
            tx.nonce()
        ))
    }).collect();

    while hashes.len() > 1 {
        hashes = hashes.chunks(2).map(|pair| {
            let left = &pair[0];
            let right = pair.get(1).unwrap_or(left);
            sha256_hex(&format!("{}{}", left, right))
        }).collect();
    }
    hashes.remove(0)
}
