//merklee.rs
use crate::transactions;
use sha2::Digest;
use transactions::Transaction;

pub fn merklee_root(transactions: &[Transaction]) -> String {
    if transactions.is_empty() {
        return String::new();
    }
    let mut hashes: Vec<String> = transactions.iter().map(|tx| {
        let contenido = format!("{}{}{}", hex::encode(tx.sender), hex::encode(tx.receiver), tx.amount);
        let mut hasher = sha2::Sha256::new();
        hasher.update(contenido.as_bytes());
        let result = hasher.finalize();
        format!("{:x}", result)
    }).collect();

    while hashes.len() > 1 {
        let mut new_hashes = Vec::new();
        for i in (0..hashes.len()).step_by(2) {
            let left = &hashes[i];
            let right = if i + 1 < hashes.len() { &hashes[i + 1] } else { left };
            let combined = format!("{}{}", left, right);
            let mut hasher = sha2::Sha256::new();
            hasher.update(combined.as_bytes());
            let result = hasher.finalize();
            new_hashes.push(format!("{:x}", result));
        }
        hashes = new_hashes;
    }
    hashes[0].clone()
}