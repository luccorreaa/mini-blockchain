use std::vec;

use sha2::{Sha256, Digest};
fn main(){
    let mut blockchain = Blockchain::new_blockchain();
    blockchain.add_block("chau");
    println!("Nueva Blockchain {:?}", blockchain);
    blockchain.cadena[0].datos = String::from("datos modified");
    println!("\nBlock - modificada {:?}", blockchain);
    println!("Es valido? {:?}", blockchain.validar());

}


#[derive(Debug)]
struct Block{
    index: u32,
    datos: String,
    hash_previo: String,
    hash: String

}
impl Block{
    fn calcular_hash(&self)->String{
        let mut hasher = Sha256::new();
        let contenido = format!("{}{}{}", self.index, self.datos, self.hash_previo);
        hasher.update(&contenido.as_bytes());
        let result = hasher.finalize();
        format!("{:x}",result)
    }
    fn new(index: u32, datos: &str, hash_previo: &str) -> Block{
        let mut bloque = Block{
            index: index,
            datos: String::from(datos),
            hash_previo: String::from(hash_previo),
            hash: String::new()
        };
        bloque.hash = bloque.calcular_hash();
        bloque
    }
}

#[derive(Debug)]
struct Blockchain{
    cadena: Vec<Block>
}

impl Blockchain {
    fn new_blockchain() ->Blockchain{
        let bloque = Block::new(0, "Hola", "");
        let blockchain = Blockchain{
            cadena: vec![bloque]
        };
        blockchain

    }
    fn add_block(&mut self, datos: &str){
        match self.cadena.last(){
            Some(bloque) => {
                let nuevo_bloque = Block::new(bloque.index + 1, datos, &bloque.hash);
                self.cadena.push(nuevo_bloque);
            }
            None => {}
        }
    }
    fn validar(&self)->bool{
        for (i, bloque) in self.cadena.iter().enumerate() {
            if bloque.hash != bloque.calcular_hash() {
                return false;
            }
            if i > 0 {
                let anterior = &self.cadena[i - 1];
                if bloque.hash_previo != anterior.hash {
                    return false;
                }
            }
        }
        true
    }
}