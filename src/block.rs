use sha2::{Sha256, Digest};
use std::time::{SystemTime};
#[derive(Debug)]
pub struct Block{
    index: u32,
    datos: String,
    hash_previo: String,
    hash: String,
    timestamp: u64,
    firma: Option<[u8; 64]>,
    autor: Option<[u8; 32]>
}
impl Block{
    pub fn calcular_hash(&self)->String{
        let mut hasher = Sha256::new();
        let contenido = format!("{}{}{}{}", self.index, self.datos, self.hash_previo, self.timestamp);
        hasher.update(&contenido.as_bytes());
        let result = hasher.finalize();
        format!("{:x}",result)
    }
    pub fn new(index: u32, datos: &str, hash_previo: &str) -> Block{
        let mut bloque = Block{
            index: index,
            datos: String::from(datos),
            hash_previo: String::from(hash_previo),
            hash: String::new(),
            timestamp: SystemTime::now().duration_since(SystemTime::UNIX_EPOCH).unwrap().as_secs(),
            firma: None,
            autor: None
        };
        bloque.hash = bloque.calcular_hash();
        bloque
    }
    fn firmar(&mut self, firma: [u8; 64], autor: [u8; 32]){
        self.firma = Some(firma);
        self.autor = Some(autor);
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

}