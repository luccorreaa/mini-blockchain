//blockchain.rs
use ed25519_dalek::SigningKey;
use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use serde::{Serialize, Deserialize};
use crate::merkle::merkle_root;
use crate::block::Block;
use crate::transactions::Transaction;
use hex;
#[derive(Debug, Serialize, Deserialize)]
pub struct Blockchain {
    cadena: Vec<Block>,
    #[serde(default)]
    mempool: Vec<Transaction>
}

impl Blockchain {
    pub fn new_blockchain() -> Blockchain {
        let bloque = Block::new(0, vec![], "");
        Blockchain { cadena: vec![bloque] 
        , mempool: vec![] }
    }

    pub fn add_block(&mut self, transactions: Vec<Transaction>) {
        if let Some(bloque) = self.cadena.last() {
            let nuevo_bloque = Block::new(bloque.index() + 1, transactions, bloque.hash());
            self.cadena.push(nuevo_bloque);
        }
    }
    pub fn add_transaction(&mut self, transaction: Transaction) {
        self.mempool.push(transaction);
    }
    
    pub fn minar(&mut self, dificultad: usize){
        if let Some(bloque) = self.cadena.last() {
            let txs = std::mem::take(&mut self.mempool);
            let mut nuevo_bloque = Block::new(bloque.index() + 1, txs, bloque.hash());
            nuevo_bloque.minar(dificultad);
            self.cadena.push(nuevo_bloque);
        }
    }

    pub fn validar(&self) -> bool {
        for (i, bloque) in self.cadena.iter().enumerate() {
            if bloque.hash() != bloque.calcular_hash() {
                return false;
            }
            if i > 0 {
                let anterior = &self.cadena[i - 1];
                if bloque.prev_hash() != anterior.hash() {
                    return false;
                }
            }

            // Verificar firma de cada transacción
            for tx in bloque.transactions() {
                if tx.sender == [0u8; 32] {
                    continue; // coinbase: no requiere firma
                }
                if let Some(firma_bytes) = &tx.firma {
                    let sig_array: [u8; 64] = match firma_bytes.as_slice().try_into() {
                        Ok(arr) => arr,
                        Err(_) => return false,
                    };
                    let signature = Signature::from_bytes(&sig_array);
                    let contenido = format!(
                        "{}{}{}{}",
                        hex::encode(tx.sender),
                        hex::encode(tx.receiver),
                        tx.amount,
                        tx.nonce
                    );
                    match VerifyingKey::from_bytes(&tx.sender) {
                        Ok(verifying_key) => {
                            if verifying_key.verify(contenido.as_bytes(), &signature).is_err() {
                                return false;
                            }
                        }
                        Err(_) => return false,
                    }
                } else {
                    return false; // tx sin firma → inválida
                }
            }

            // Verificar firma del bloque (si tiene)
            if let (Some(firma_bytes), Some(autor_bytes)) = (bloque.signature(), bloque.author()) {
                let sig_array: [u8; 64] = match firma_bytes.as_slice().try_into() {
                    Ok(arr) => arr,
                    Err(_) => return false,
                };
                let signature = Signature::from_bytes(&sig_array);
                let contenido = format!(
                    "{}{}{}{}",
                    bloque.index(),
                    merkle_root(bloque.transactions()),
                    bloque.prev_hash(),
                    bloque.timestamp()
                );
                match VerifyingKey::from_bytes(&autor_bytes) {
                    Ok(verifying_key) => {
                        if verifying_key.verify(contenido.as_bytes(), &signature).is_err() {
                            return false;
                        }
                    }
                    Err(_) => return false,
                }
            }
        }
        true
    }

    pub fn firmar_bloque(&mut self, index: usize, signing_key: &SigningKey) {
        if let Some(bloque) = self.cadena.iter_mut().find(|b| b.index() as usize == index) {
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

    pub fn cadena(&self) -> &[Block] {
        &self.cadena
    }
    #[cfg(test)]
    pub fn corromper_bloque(&mut self, index: usize) {
    if let Some(bloque) = self.cadena.iter_mut().find(|b| b.index() as usize == index) {
        bloque.corromper();
    }
}

}


#[cfg(test)]
mod tests{

use super::*;

    #[test]
    fn cadena_corrompida_no_es_valida() {
        let mut blockchain = Blockchain::new_blockchain();

        blockchain.add_block(vec![]);
        blockchain.add_block(vec![]);
        assert!(blockchain.validar());
        blockchain.corromper_bloque(1);
        assert!(!blockchain.validar());
    }

    #[test]
    fn validar_no_panic_con_firma_de_longitud_incorrecta() {
        let mut blockchain = Blockchain::new_blockchain();
        blockchain.add_block(vec![]);
        if let Some(bloque) = blockchain.cadena.iter_mut().find(|b| b.index() == 1) {
            bloque.set_firma_test(vec![0u8; 10]); // longitud incorrecta: antes causaba panic
            bloque.set_autor_test([1u8; 32]); // necesario para activar la verificación de firma
        }
        assert!(!blockchain.validar()); // no debe paniquear, debe retornar false
    }
}
