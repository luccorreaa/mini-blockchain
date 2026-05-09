mod block;
mod blockchain;
mod transactions;
use crate::blockchain::Blockchain;
use rand::rngs::OsRng;
use ed25519_dalek::SigningKey;
use rand::RngCore;
fn main(){
    let mut blockchain = Blockchain::new_blockchain();
    
    let mut secret1 = [0u8; 32];
    let mut secret2 = [0u8; 32];
    OsRng.fill_bytes(&mut secret1);
    OsRng.fill_bytes(&mut secret2);
    
    let bob = SigningKey::from_bytes(&secret1);
    let alicia = SigningKey::from_bytes(&secret2);

    let bob_pubkey = bob.verifying_key().to_bytes();
    let alicia_pubkey = alicia.verifying_key().to_bytes();

    let mut tx1 = transactions::Transaction::new(bob_pubkey, alicia_pubkey, 50);
    let mut tx2 = transactions::Transaction::new(alicia_pubkey, bob_pubkey, 30);
    
    tx1.firmar(&bob);
    tx2.firmar(&alicia);

    blockchain.add_block(vec![tx1, tx2]);
    
    blockchain.firmar_bloque(0, &alicia);

    blockchain.firmar_bloque(1, &bob);

    blockchain.guardar("blockchain.json").expect("Error al guardar la blockchain");
    let blockchain_cargada = Blockchain::cargar("blockchain.json").expect("Error al cargar la blockchain");
    println!("Blockchain cargada: {:?}\n", blockchain_cargada);

    println!("Blockchain válida: {}", blockchain.validar());
}




