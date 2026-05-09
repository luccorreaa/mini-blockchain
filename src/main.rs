mod block;
mod blockchain;
mod transactions;
use crate::blockchain::Blockchain;
use rand::rngs::OsRng;
use ed25519_dalek::SigningKey;
use rand::RngCore;
fn main(){
    let mut blockchain = Blockchain::new_blockchain();
    
    let mut tx = transactions::Transaction::new([0u8; 32], [1u8; 32], 100);
    
    let mut secret = [0u8; 32];
    
    OsRng.fill_bytes(&mut secret);
    
    let signing_key = SigningKey::from_bytes(&secret);
    
    tx.firmar(&signing_key);
    
    blockchain.add_block(vec![tx]);
    
    blockchain.firmar_bloque(0, &signing_key);

    blockchain.firmar_bloque(1, &signing_key);

    blockchain.guardar("blockchain.json").expect("Error al guardar la blockchain");
    let blockchain_cargada = Blockchain::cargar("blockchain.json").expect("Error al cargar la blockchain");
    println!("Blockchain cargada: {:?}", blockchain_cargada);

}




