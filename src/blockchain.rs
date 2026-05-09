use sha2::Digest;
use crate::block::Block;
use crate::transactions::Transaction;
use ed25519_dalek::SigningKey;
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use serde::{Serialize, Deserialize};

#[derive(Debug)]
#[derive(Serialize, Deserialize)]
pub struct Blockchain {
    cadena: Vec<Block>,
}

impl Blockchain {
    pub fn new_blockchain() -> Blockchain {
        let bloque = Block::new(0, vec![], "");
        Blockchain { cadena: vec![bloque] }
    }

    pub fn add_block(&mut self, transactions: Vec<Transaction>) {
        match self.cadena.last() {
            Some(bloque) => {
                let nuevo_bloque = Block::new(bloque.get_index() + 1, transactions, &bloque.get_hash());
                self.cadena.push(nuevo_bloque);
            }
            None => {}
        }
    }

    pub fn validar(&self) -> bool {
        for (i, bloque) in self.cadena.iter().enumerate() {
            if bloque.get_hash() != bloque.calcular_hash() {
                return false;
            }
            if i > 0 {
                let anterior = &self.cadena[i - 1];
                if bloque.get_hash_previo() != anterior.get_hash() {
                    return false;
                }
            }
            if let (Some(firma_bytes), Some(autor_bytes)) = (bloque.get_firma(), bloque.get_autor()) {
                let signature = Signature::from_bytes(&firma_bytes.as_slice().try_into().unwrap());
                let contenido = format!(
                    "{}{}{}{}",
                    bloque.get_index(),
                    bloque.get_datos().iter().map(|tx| format!("{}{}{}", hex::encode(tx.sender), hex::encode(tx.receiver), tx.amount)).collect::<Vec<String>>().join(""),
                    bloque.get_hash_previo(),
                    bloque.get_timestamp()
                );
                if let Ok(verifying_key) = VerifyingKey::from_bytes(&autor_bytes) {
                    if verifying_key.verify(contenido.as_bytes(), &signature).is_err() {
                        return false;
                    }
                }
            }
        }
        true
    }

    pub fn firmar_bloque(&mut self, index: usize, signing_key: &SigningKey) {
        if let Some(bloque) = self.cadena.iter_mut().find(|b| b.get_index() as usize == index) {
            bloque.firmar(signing_key);
        }
    }
    pub fn guardar(&self, path: &str) -> Result<(), Box<dyn std::error::Error>> {
        let json = serde_json::to_string_pretty(&self)?;
        std::fs::write(path, json)?;
        Ok(())
    }
    pub fn cargar(path: &str) -> Result<Blockchain, Box<dyn std::error::Error>>{
        let json = std::fs::read_to_string(path)?;
        let blockchain: Blockchain = serde_json::from_str(&json)?;
        Ok(blockchain)
    }

    pub fn get_cadena(&self) -> &[Block] {
        &self.cadena
    }
}
