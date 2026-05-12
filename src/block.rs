use ed25519_dalek::SigningKey;
use ed25519_dalek::Signer;
use sha2::{Sha256, Digest};
use std::time::SystemTime;
use crate::transactions::Transaction;
use serde::{Serialize, Deserialize};
use crate::merkle::merkle_root;

#[derive(Debug, Serialize, Deserialize)]
pub struct Block {
    index: u32,
    transacciones: Vec<Transaction>,
    hash_previo: String,
    hash: String,
    timestamp: u64,
    firma: Option<Vec<u8>>,
    autor: Option<[u8; 32]>,
    nonce: u64,
}

impl Block {
    pub fn calcular_hash(&self) -> String {
        let mut hasher = Sha256::new();
        let contenido = format!("{}{}{}{}{}", self.index, merkle_root(&self.transacciones), self.hash_previo, self.timestamp, self.nonce);
        hasher.update(contenido.as_bytes());
        format!("{:x}", hasher.finalize())
    }

    pub fn new(index: u32, transaction: Vec<Transaction>, hash_previo: &str) -> Block {
        let mut bloque = Block {
            index,
            transacciones: transaction,
            hash_previo: String::from(hash_previo),
            hash: String::new(),
            timestamp: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
            firma: None,
            autor: None,
            nonce: 0,
        };
        bloque.hash = bloque.calcular_hash();
        bloque
    }

    pub fn firmar(&mut self, signing_key: &SigningKey) {
        let contenido = format!("{}{}{}{}", self.index, merkle_root(&self.transacciones), self.hash_previo, self.timestamp);
        let signature = signing_key.sign(contenido.as_bytes());
        self.firma = Some(signature.to_bytes().to_vec());
        self.autor = Some(signing_key.verifying_key().to_bytes());
    }

    pub fn minar(&mut self, dificultad: usize) {
        let objetivo = "0".repeat(dificultad);
        while !self.hash.starts_with(&objetivo) {
            self.nonce += 1;
            self.hash = self.calcular_hash();
        }
    }

    pub fn hash(&self) -> &str { &self.hash }
    pub fn prev_hash(&self) -> &str { &self.hash_previo }
    pub fn index(&self) -> u32 { self.index }
    pub fn timestamp(&self) -> u64 { self.timestamp }
    pub fn transactions(&self) -> &[Transaction] { &self.transacciones }
    pub fn signature(&self) -> &Option<Vec<u8>> { &self.firma }
    pub fn author(&self) -> Option<[u8; 32]> { self.autor }

    #[cfg(test)]
    pub fn corromper(&mut self) {
        self.hash = "hash_corrupto".to_string();
    }

    #[cfg(test)]
    pub fn set_firma_test(&mut self, firma: Vec<u8>) {
        self.firma = Some(firma);
    }

    #[cfg(test)]
    pub fn set_autor_test(&mut self, autor: [u8; 32]) {
        self.autor = Some(autor);
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_es_consistente_al_recalcular() {
        let block = Block::new(0, vec![], "0");
        assert_eq!(block.hash(), block.calcular_hash());
    }

    #[test]
    fn hash_cambia_al_agregar_transaccion() {
        let mut block = Block::new(0, vec![], "0");
        let hash_original = block.calcular_hash();
        block.transacciones.push(Transaction::new([0u8; 32], [1u8; 32], 100));
        assert_ne!(hash_original, block.calcular_hash());
    }
}
