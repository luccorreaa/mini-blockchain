use std::vec;
use sha2::Digest;
use crate::block::Block;

#[derive(Debug)]
pub struct Blockchain{
    cadena: Vec<Block>
}

impl Blockchain {
    pub fn new_blockchain() ->Blockchain{
        let bloque = Block::new(0, "Hola", "", );
        let blockchain = Blockchain{
            cadena: vec![bloque]
        };
        blockchain

    }
    pub fn add_block(&mut self, datos: &str){
        match self.cadena.last(){
            Some(bloque) => {
                let nuevo_bloque = Block::new(bloque.get_index()+ 1, datos, &bloque.get_hash());
                self.cadena.push(nuevo_bloque);
            }
            None => {}
        }
    }
    pub fn validar(&self)->bool{
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
        }
        true
    }
    pub fn get_cadena(&self)->&[Block]{
        &self.cadena
    }
}