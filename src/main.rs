//main.rs
mod block;
mod blockchain;
mod transactions;
mod merklee;
mod cli;
mod wallet;
use crate::cli::Cli;
use clap::{Parser, Subcommand};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
fn main(){
    let cli = Cli::parse();
    match cli.command {
        cli::Command::NewWallet => {
            let mut secret = [0u8; 32];
            OsRng.fill_bytes(&mut secret);
            let signing_key = SigningKey::from_bytes(&secret);
            let pubkey = signing_key.verifying_key().to_bytes();
            let wallet = wallet::Wallet::new(secret, pubkey);
            wallet.guardar("wallet.json").expect("Error al guardar la wallet");
            println!("Generando nueva wallet...");
            println!("Clave pública: {}", hex::encode(pubkey));
        }
        cli::Command::ShowChain => {
            let blockchain = blockchain::Blockchain::cargar("blockchain.json").unwrap_or_else(|_| blockchain::Blockchain::new_blockchain());
            println!("Mostrando la cadena de bloques...");

            for bloque in blockchain.get_cadena() {
                println!("Bloque {}: Hash: {}, Hash Previo: {}, Timestamp: {}, Transacciones: {}", bloque.get_index(), bloque.get_hash(), bloque.get_hash_previo(), bloque.get_timestamp(), bloque.get_datos().len());
            }
        }
        cli::Command::Validate => {
            let blockchain = blockchain::Blockchain::cargar("blockchain.json").unwrap_or_else(|_| blockchain::Blockchain::new_blockchain());
            
            println!("Validando la cadena de bloques...");
            println!("La cadena de bloques es válida: {}", blockchain.validar());
        }
        cli::Command::Send { from, to, amount } => {
            let mut blockchain = blockchain::Blockchain::cargar("blockchain.json").unwrap_or_else(|_| blockchain::Blockchain::new_blockchain());
            let mut tx = transactions::Transaction::new(hex::decode(&from).expect("Clave pública inválida").try_into().expect("Clave pública debe ser de 32 bytes"), hex::decode(&to).expect("Clave pública inválida").try_into().expect("Clave pública debe ser de 32 bytes"), amount);
            println!("Enviando {} desde {} a {}...", amount, from, to);
            let wallet = wallet::Wallet::cargar("wallet.json").expect("Error al cargar la wallet");
            let signing_key = SigningKey::from_bytes(&wallet.secret);
            tx.firmar(&signing_key);
            blockchain.add_block(vec![tx]);
            blockchain.guardar("blockchain.json").expect("Error al guardar la blockchain");
        }
    }
}




