//! Blockchain: an ordered chain of blocks and a pending-transaction mempool.

use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use serde::{Serialize, Deserialize};
use crate::chain::merkle::merkle_root;
use crate::chain::block::Block;
use crate::crypto::transaction::Transaction;
use crate::types::{Hash, PublicKey};
use crate::error::{TransactionError, ChainError, ChainResult};

/// An append-only chain of [`Block`]s with a pending-transaction mempool.
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
    pub fn new() -> Self { Self::with_difficulty(2) }

    pub fn with_difficulty(difficulty: usize) -> Self {
        let genesis = Block::new(0, vec![], &Hash::empty());
        Self { chain: vec![genesis], mempool: vec![], difficulty }
    }

    pub fn add_block(&mut self, transactions: Vec<Transaction>) {
        if let Some(tip) = self.chain.last() {
            let new_block = Block::new(tip.index() + 1, transactions, tip.hash());
            self.chain.push(new_block);
        }
    }

    pub fn balance_of(&self, pubkey: &PublicKey) -> u64 {
        let mut balance = 0u64;
        for block in &self.chain {
            for tx in block.transactions() {
                if !tx.sender().is_coinbase() && tx.sender() == *pubkey {
                    balance = balance.saturating_sub(tx.amount());
                }
                if tx.receiver() == *pubkey {
                    balance = balance.saturating_add(tx.amount());
                }
            }
        }
        for tx in &self.mempool {
            if !tx.sender().is_coinbase() && tx.sender() == *pubkey {
                balance = balance.saturating_sub(tx.amount());
            }
        }
        balance
    }

    pub fn add_coinbase(&mut self, miner: PublicKey, reward: u64) {
        let coinbase = Transaction::new(PublicKey::coinbase(), miner, reward);
        self.mempool.insert(0, coinbase);
    }

    pub fn add_transaction(&mut self, transaction: Transaction) -> ChainResult<()> {
        if !transaction.sender().is_coinbase() {
            let available = self.balance_of(&transaction.sender());
            if available < transaction.amount() {
                return Err(ChainError::InsufficientBalance {
                    available,
                    required: transaction.amount(),
                });
            }
        }
        self.mempool.push(transaction);
        Ok(())
    }

    pub fn difficulty(&self) -> usize { self.difficulty }

    pub fn tip(&self) -> Option<(u32, Hash)> {
        self.chain.last().map(|b| (b.index(), b.hash().clone()))
    }

    pub fn take_mempool(&mut self) -> Vec<Transaction> {
        std::mem::take(&mut self.mempool)
    }

    pub fn push_block(&mut self, block: Block) {
        self.chain.push(block);
    }

    pub fn mine(&mut self) {
        if let Some((tip_index, tip_hash)) = self.tip() {
            let txs = self.take_mempool();
            let mut new_block = Block::new(tip_index + 1, txs, &tip_hash);
            new_block.mine(self.difficulty);
            self.chain.push(new_block);
        }
    }

    /// Validates the entire chain: hash integrity, block linkage, and signatures.
    pub fn validate(&self) -> bool {
        self.chain.iter().enumerate().all(|(i, block)| self.validate_block(i, block))
    }

    fn validate_block(&self, i: usize, block: &Block) -> bool {
        if block.hash() != &block.compute_hash() { return false; }
        if i > 0 && block.prev_hash() != self.chain[i - 1].hash() { return false; }
        for tx in block.transactions() {
            if tx.sender().is_coinbase() { continue; }
            if Self::verify_tx_signature(tx).is_err() { return false; }
        }
        self.verify_block_signature(block).is_ok()
    }

    fn verify_tx_signature(tx: &Transaction) -> Result<(), TransactionError> {
        let sig_bytes = tx.signature().ok_or(TransactionError::InvalidSignature)?;
        let sig_array: [u8; 64] = sig_bytes.try_into()
            .map_err(|_| TransactionError::InvalidSignatureLength)?;
        let signature = Signature::from_bytes(&sig_array);
        let content = format!(
            "{}{}{}{}",
            hex::encode(tx.sender().as_bytes()),
            hex::encode(tx.receiver().as_bytes()),
            tx.amount(),
            tx.nonce()
        );
        let verifying_key = VerifyingKey::from_bytes(tx.sender().as_bytes())
            .map_err(|_| TransactionError::InvalidPublicKey)?;
        verifying_key.verify(content.as_bytes(), &signature)
            .map_err(|_| TransactionError::InvalidSignature)
    }

    fn verify_block_signature(&self, block: &Block) -> Result<(), ChainError> {
        let (sig_bytes, author) = match (block.signature(), block.author()) {
            (Some(s), Some(a)) => (s, a),
            _ => return Ok(()),
        };
        let sig_array: [u8; 64] = sig_bytes.try_into()
            .map_err(|_| TransactionError::InvalidSignatureLength)?;
        let signature = Signature::from_bytes(&sig_array);
        let content = format!(
            "{}{}{}{}",
            block.index(), merkle_root(block.transactions()), block.prev_hash(), block.timestamp()
        );
        let verifying_key = VerifyingKey::from_bytes(author.as_bytes())
            .map_err(|_| TransactionError::InvalidPublicKey)?;
        verifying_key.verify(content.as_bytes(), &signature)
            .map_err(|_| TransactionError::InvalidSignature)?;
        Ok(())
    }

    pub fn sign_block(&mut self, index: usize, signing_key: &ed25519_dalek::SigningKey) {
        if let Some(block) = self.chain.iter_mut().find(|b| b.index() as usize == index) {
            block.sign(signing_key);
        }
    }

    pub fn save(&self, path: &str) -> ChainResult<()> {
        std::fs::write(path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }

    pub fn load(path: &str) -> ChainResult<Blockchain> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    pub fn chain(&self) -> &[Block] { &self.chain }

    #[cfg(test)]
    pub fn corrupt_block(&mut self, index: usize) {
        if let Some(block) = self.chain.iter_mut().find(|b| b.index() as usize == index) {
            block.corrupt();
        }
    }
}

impl Default for Blockchain {
    fn default() -> Self { Self::new() }
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
        assert_eq!(blockchain.balance_of(&miner), 50);
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
            block.set_author_test(PublicKey([1u8; 32]));
        }
        assert!(!blockchain.validate());
    }
}
