//main.rs
mod cli;
use crate::cli::{Cli, Command};
use clap::Parser;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
use mini_blockchain::blockchain::Blockchain;
use mini_blockchain::wallet::Wallet;
use mini_blockchain::transactions::Transaction;

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::NewWallet => {
            let mut secret = [0u8; 32];
            OsRng.fill_bytes(&mut secret);
            let signing_key = SigningKey::from_bytes(&secret);
            let pubkey = signing_key.verifying_key().to_bytes();
            let wallet = Wallet::new(secret, pubkey);
            wallet.guardar("wallet.json").expect("Error al guardar la wallet");
            println!("Generando nueva wallet...");
            println!("Clave pública: {}", hex::encode(pubkey));
        }
        Command::ShowChain => {
            let blockchain = Blockchain::cargar("blockchain.json").unwrap_or_else(|_| Blockchain::new_blockchain());
            println!("Mostrando la cadena de bloques...");
            for bloque in blockchain.get_cadena() {
                println!("Bloque {}: Hash: {}, Hash Previo: {}, Timestamp: {}, Transacciones: {}", bloque.index(), bloque.hash(), bloque.prev_hash(), bloque.timestamp(), bloque.transactions().len());
            }
        }
        Command::Validate => {
            let blockchain = Blockchain::cargar("blockchain.json").unwrap_or_else(|_| Blockchain::new_blockchain());
            println!("Validando la cadena de bloques...");
            println!("La cadena de bloques es válida: {}", blockchain.validar());
        }
        Command::Send { from, to, amount } => {
            let mut blockchain = Blockchain::cargar("blockchain.json").unwrap_or_else(|_| Blockchain::new_blockchain());
            let mut tx = Transaction::new(
                hex::decode(&from).expect("Clave pública inválida").try_into().expect("32 bytes"),
                hex::decode(&to).expect("Clave pública inválida").try_into().expect("32 bytes"),
                amount
            );
            println!("Enviando {} desde {} a {}...", amount, from, to);
            let wallet = Wallet::cargar("wallet.json").expect("Error al cargar la wallet");
            let signing_key = SigningKey::from_bytes(&wallet.secret);
            tx.firmar(&signing_key);
            blockchain.add_block(vec![tx]);
            blockchain.guardar("blockchain.json").expect("Error al guardar la blockchain");
        }
    }
}