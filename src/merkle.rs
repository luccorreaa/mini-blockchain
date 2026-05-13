//! Merkle tree root computation for transaction sets.
//!
//! Provides [`merkle_root`], which computes the SHA-256 Merkle root of a slice of
//! transactions. The root is included in each block's hash to commit to its complete
//! transaction set without storing all transactions in the header.

use crate::transactions;
use sha2::Digest;
use transactions::Transaction;

/// Computes the SHA-256 Merkle root of `transactions`.
///
/// Returns an empty string when `transactions` is empty.
/// Odd-length layers duplicate the last hash before combining pairs.
pub fn merkle_root(transactions: &[Transaction]) -> String {
    if transactions.is_empty() {
        return String::new();
    }
    let mut hashes: Vec<String> = transactions.iter().map(|tx| {
        let content = format!(
            "{}{}{}{}",
            hex::encode(tx.sender),
            hex::encode(tx.receiver),
            tx.amount,
            tx.nonce
        );
        let mut hasher = sha2::Sha256::new();
        hasher.update(content.as_bytes());
        format!("{:x}", hasher.finalize())
    }).collect();

    while hashes.len() > 1 {
        let mut new_hashes = Vec::new();
        for i in (0..hashes.len()).step_by(2) {
            let left = &hashes[i];
            let right = if i + 1 < hashes.len() { &hashes[i + 1] } else { left };
            let combined = format!("{}{}", left, right);
            let mut hasher = sha2::Sha256::new();
            hasher.update(combined.as_bytes());
            new_hashes.push(format!("{:x}", hasher.finalize()));
        }
        hashes = new_hashes;
    }
    hashes[0].clone()
}
