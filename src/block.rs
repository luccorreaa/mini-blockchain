use ed25519_dalek::SigningKey;
use ed25519_dalek::Signer;
use sha2::{Sha256, Digest};
use std::time::{SystemTime};
use std::vec;
use crate::transactions::Transaction;
use serde::{Serialize, Deserialize};
use serde_big_array::BigArray;
use crate::merklee::merklee_root;
#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Block{
    index: u32,
    transacciones: Vec<Transaction>,
    hash_previo: String,
    hash: String,
    timestamp: u64,
    pub firma: Option<Vec<u8>>,
    autor: Option<[u8; 32]>
}

impl Block{
    pub fn calcular_hash(&self)->String{
        let mut hasher = Sha256::new();
        let contenido = format!("{}{}{}{}", self.index, merklee_root(&self.transacciones), self.hash_previo, self.timestamp);
        hasher.update(&contenido.as_bytes());
        let result = hasher.finalize();
        format!("{:x}",result)
    }

    pub fn new(index: u32, transaction: Vec<Transaction>, hash_previo: &str) -> Block{
        let mut bloque = Block{
            index: index,
            transacciones: transaction,
            hash_previo: String::from(hash_previo),
            hash: String::new(),
            timestamp: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
            firma: None,
            autor: None
        };
        bloque.hash = bloque.calcular_hash();
        bloque
    }   

    pub fn firmar(&mut self, signing_key: &SigningKey){
        let contenido = format!("{}{}{}{}", self.index, merklee_root(&self.transacciones), self.hash_previo, self.timestamp);
        let signature = signing_key.sign(contenido.as_bytes());
        self.firma = Some(signature.to_bytes().to_vec());
        self.autor = Some(signing_key.verifying_key().to_bytes());
    }

    pub fn get_hash(&self)->&str{
        &self.hash
    }
    pub fn get_hash_previo(&self)->&str{
        &self.hash_previo
    }
    pub fn get_index(&self)->u32{
        self.index
    }
    pub fn get_timestamp(&self)->u64{
        self.timestamp
    }
    pub fn get_datos(&self)->&[Transaction]{
        &self.transacciones
    }
    pub fn get_firma(&self) -> &Option<Vec<u8>>{
    &self.firma
    }

    pub fn get_autor(&self) -> Option<[u8; 32]> {
    self.autor
}
}