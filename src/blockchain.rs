//! Blockchain: an ordered chain of blocks and a pending-transaction mempool.
//!
//! [`Blockchain`] owns the canonical chain and the set of unconfirmed transactions
//! (the *mempool*). It enforces balance checks on incoming transactions, orchestrates
//! Proof-of-Work mining, and validates the integrity of the entire chain including
//! transaction and block signatures.

use ed25519_dalek::SigningKey;
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use serde::{Serialize, Deserialize};
use crate::merkle::merkle_root;
use crate::block::Block;
use crate::crypto::transaction::Transaction;
use crate::types::PublicKey;
use hex;

/// An append-only chain of [`Block`]s with a pending-transaction mempool.
///
/// The chain always contains at least a genesis block (index 0, empty transactions).
/// `difficulty` controls how many leading zero hex digits the Proof-of-Work hash must have.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Blockchain {
    chain: Vec<Block>,
    #[serde(default)]
    mempool: Vec<Transaction>,
    #[serde(default = "default_difficulty")]
    difficulty: usize,
}

fn default_difficulty() -> usize { 2 }

impl Blockchain {
    /// Creates a new blockchain with default mining difficulty (2).
    pub fn new() -> Blockchain {
        Blockchain::with_difficulty(2)
    }

    /// Creates a new blockchain with the specified mining `difficulty`.
    pub fn with_difficulty(difficulty: usize) -> Blockchain {
        let genesis = Block::new(0, vec![], "");
        Blockchain { chain: vec![genesis], mempool: vec![], difficulty }
    }

    /// Appends a new unmined block containing `transactions` to the chain.
    ///
    /// Prefer [`Blockchain::mine`] for normal block production.
    pub fn add_block(&mut self, transactions: Vec<Transaction>) {
        if let Some(tip) = self.chain.last() {
            let new_block = Block::new(tip.index() + 1, transactions, tip.hash());
            self.chain.push(new_block);
        }
    }

    /// Returns the available balance of `pubkey`, accounting for pending mempool spends.
    ///
    /// Coinbase inputs (all-zero sender) are never subtracted from any balance.
    /// Pending sends in the mempool are subtracted to reflect available (not just confirmed) funds.
    pub fn balance_of(&self, pubkey: &[u8; 32]) -> u64 {
        let mut balance = 0u64;
        for block in &self.chain {
            for tx in block.transactions() {
                if !tx.sender().is_coinbase() && tx.sender().as_bytes() == pubkey {
                    balance = balance.saturating_sub(tx.amount());
                }
                if tx.receiver().as_bytes() == pubkey {
                    balance = balance.saturating_add(tx.amount());
                }
            }
        }
        // Subtract committed mempool spends so callers see available, not just confirmed, balance.
        for tx in &self.mempool {
            if !tx.sender().is_coinbase() && tx.sender().as_bytes() == pubkey {
                balance = balance.saturating_sub(tx.amount());
            }
        }
        balance
    }

    /// Inserts a coinbase transaction (miner reward) at the front of the mempool.
    ///
    /// Coinbase transactions use the all-zero sender and bypass balance checks.
    pub fn add_coinbase(&mut self, miner: PublicKey, reward: u64) {
        let coinbase = Transaction::new(PublicKey::coinbase(), miner, reward);
        self.mempool.insert(0, coinbase);
    }

    /// Adds a signed transaction to the mempool after verifying the sender has sufficient balance.
    ///
    /// # Errors
    ///
    /// Returns an error string if the sender's available balance is less than `transaction.amount`.
    pub fn add_transaction(&mut self, transaction: Transaction) -> Result<(), String> {
        if !transaction.sender().is_coinbase() {
            let available = self.balance_of(transaction.sender().as_bytes());
            if available < transaction.amount() {
                return Err(format!(
                    "Insufficient balance: available {}, required {}",
                    available, transaction.amount()
                ));
            }
        }
        self.mempool.push(transaction);
        Ok(())
    }

    /// Returns the current mining difficulty (number of required leading zero hex digits).
    pub fn difficulty(&self) -> usize { self.difficulty }

    /// Returns the index and hash of the most recent block, or `None` if the chain is empty.
    pub fn tip(&self) -> Option<(u32, String)> {
        self.chain.last().map(|b| (b.index(), b.hash().to_string()))
    }

    /// Drains and returns all pending transactions from the mempool.
    pub fn take_mempool(&mut self) -> Vec<Transaction> {
        std::mem::take(&mut self.mempool)
    }

    /// Appends a pre-built block directly to the chain.
    ///
    /// Used during P2P sync when receiving a block from a peer.
    pub fn push_block(&mut self, block: Block) {
        self.chain.push(block);
    }

    /// Mines a new block from the current mempool and appends it to the chain.
    ///
    /// Drains the mempool, creates a block linked to the current tip, and runs
    /// Proof-of-Work until the hash meets the configured difficulty.
    pub fn mine(&mut self) {
        if let Some((tip_index, tip_hash)) = self.tip() {
            let txs = self.take_mempool();
            let mut new_block = Block::new(tip_index + 1, txs, &tip_hash);
            new_block.mine(self.difficulty);
            self.chain.push(new_block);
        }
    }

    /// Validates the entire chain: hash integrity, block linkage, and signatures.
    ///
    /// Returns `false` if any block's stored hash does not match its recomputed hash,
    /// if a block's `prev_hash` does not match its predecessor, if a non-coinbase
    /// transaction lacks a valid Ed25519 signature, or if a block signature is invalid.
    pub fn validate(&self) -> bool {
        for (i, block) in self.chain.iter().enumerate() {
            if block.hash() != block.compute_hash() {
                return false;
            }
            if i > 0 {
                let prev = &self.chain[i - 1];
                if block.prev_hash() != prev.hash() {
                    return false;
                }
            }

            for tx in block.transactions() {
                if tx.sender().is_coinbase() {
                    continue; // coinbase: no signature required
                }
                if let Some(sig_bytes) = tx.signature() {
                    let sig_array: [u8; 64] = match sig_bytes.try_into() {
                        Ok(arr) => arr,
                        Err(_) => return false,
                    };
                    let signature = Signature::from_bytes(&sig_array);
                    let content = format!(
                        "{}{}{}{}",
                        hex::encode(tx.sender().as_bytes()),
                        hex::encode(tx.receiver().as_bytes()),
                        tx.amount(),
                        tx.nonce()
                    );
                    match VerifyingKey::from_bytes(tx.sender().as_bytes()) {
                        Ok(verifying_key) => {
                            if verifying_key.verify(content.as_bytes(), &signature).is_err() {
                                return false;
                            }
                        }
                        Err(_) => return false,
                    }
                } else {
                    return false; // unsigned non-coinbase tx → invalid
                }
            }

            if let (Some(sig_bytes), Some(author_bytes)) = (block.signature(), block.author()) {
                let sig_array: [u8; 64] = match sig_bytes.as_slice().try_into() {
                    Ok(arr) => arr,
                    Err(_) => return false,
                };
                let signature = Signature::from_bytes(&sig_array);
                let content = format!(
                    "{}{}{}{}",
                    block.index(),
                    merkle_root(block.transactions()),
                    block.prev_hash(),
                    block.timestamp()
                );
                match VerifyingKey::from_bytes(&author_bytes) {
                    Ok(verifying_key) => {
                        if verifying_key.verify(content.as_bytes(), &signature).is_err() {
                            return false;
                        }
                    }
                    Err(_) => return false,
                }
            }
        }
        true
    }

    /// Signs the block at `index` with the given Ed25519 signing key.
    ///
    /// Does nothing if no block with that index exists.
    pub fn sign_block(&mut self, index: usize, signing_key: &SigningKey) {
        if let Some(block) = self.chain.iter_mut().find(|b| b.index() as usize == index) {
            block.sign(signing_key);
        }
    }

    /// Serialises the blockchain to a pretty-printed JSON file at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if serialisation or file I/O fails.
    pub fn save(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Deserialises a blockchain from the JSON file at `path`.
    ///
    /// # Errors
    ///
    /// Returns an error if the file cannot be read or parsed.
    pub fn load(path: &str) -> Result<Blockchain, Box<dyn std::error::Error>> {
        let json = std::fs::read_to_string(path)?;
        let blockchain: Blockchain = serde_json::from_str(&json)?;
        Ok(blockchain)
    }

    /// Returns a slice of all blocks in the chain.
    pub fn chain(&self) -> &[Block] {
        &self.chain
    }

    #[cfg(test)]
    pub fn corrupt_block(&mut self, index: usize) {
        if let Some(block) = self.chain.iter_mut().find(|b| b.index() as usize == index) {
            block.corrupt();
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blockchain_almacena_dificultad_configurada() {
        let bc = Blockchain::with_difficulty(4);
        assert_eq!(bc.difficulty(), 4);
    }

    #[test]
    fn add_transaction_rechaza_si_saldo_insuficiente() {
        let mut blockchain = Blockchain::new();
        let tx = Transaction::new(PublicKey([1u8; 32]), PublicKey([2u8; 32]), 100);
        assert!(blockchain.add_transaction(tx).is_err());
    }

    #[test]
    fn add_coinbase_agrega_a_mempool_sin_validar_saldo() {
        let mut blockchain = Blockchain::new();
        blockchain.add_coinbase(PublicKey([3u8; 32]), 50);
        assert_eq!(blockchain.mempool.len(), 1);
    }

    #[test]
    fn balance_aumenta_tras_minar_coinbase() {
        let mut blockchain = Blockchain::new();
        let miner = PublicKey([3u8; 32]);
        blockchain.add_coinbase(miner, 50);
        blockchain.mine();
        assert_eq!(blockchain.balance_of(miner.as_bytes()), 50);
    }

    #[test]
    fn cadena_corrompida_no_es_valida() {
        let mut blockchain = Blockchain::new();
        blockchain.add_block(vec![]);
        blockchain.add_block(vec![]);
        assert!(blockchain.validate());
        blockchain.corrupt_block(1);
        assert!(!blockchain.validate());
    }

    #[test]
    fn validar_no_panic_con_firma_de_longitud_incorrecta() {
        let mut blockchain = Blockchain::new();
        blockchain.add_block(vec![]);
        if let Some(block) = blockchain.chain.iter_mut().find(|b| b.index() == 1) {
            block.set_signature_test(vec![0u8; 10]);
            block.set_author_test([1u8; 32]);
        }
        assert!(!blockchain.validate());
    }
}
