mod block;
mod blockchain;
use crate::blockchain::Blockchain;
use rand::rngs::OsRng;
use ed25519_dalek::SigningKey;
use ed25519_dalek::Signature;

fn main(){
    let mut blockchain = Blockchain::new_blockchain();
    blockchain.add_block("chau");
    println!("Nueva Blockchain {:?}", blockchain);
    println!("Es valido? {:?}", blockchain.validar());
    
    let signing_key: SigningKey = SigningKey::generate(&mut OsRng);
}




