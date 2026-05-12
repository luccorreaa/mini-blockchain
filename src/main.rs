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

fn wallet_password() -> String {
    std::env::var("WALLET_PASSWORD").unwrap_or_else(|_| "dev_password_change_me".to_string())
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::NewWallet => {
            if std::path::Path::new("wallet.json").exists() {
                eprintln!("Ya existe una wallet. Eliminá wallet.json antes de generar una nueva.");
                std::process::exit(1);
            }
            let mut secret = [0u8; 32];
            OsRng.fill_bytes(&mut secret);
            let signing_key = SigningKey::from_bytes(&secret);
            let pubkey = signing_key.verifying_key().to_bytes();
            let wallet = Wallet::new(secret, pubkey);
            wallet.guardar_cifrado("wallet.json", &wallet_password()).expect("Error al guardar la wallet");
            println!("Generando nueva wallet...");
            println!("Clave pública: {}", hex::encode(pubkey));
        }
        Command::ShowChain => {
            let blockchain = Blockchain::cargar("blockchain.json").unwrap_or_else(|_| Blockchain::new_blockchain());
            println!("Mostrando la cadena de bloques...");
            for bloque in blockchain.cadena() {
                println!("Bloque {}: Hash: {}, Hash Previo: {}, Timestamp: {}, Transacciones: {}", bloque.index(), bloque.hash(), bloque.prev_hash(), bloque.timestamp(), bloque.transactions().len());
            }
        }
        Command::Validate => {
            let blockchain = Blockchain::cargar("blockchain.json").unwrap_or_else(|_| Blockchain::new_blockchain());
            println!("Validando la cadena de bloques...");
            println!("La cadena de bloques es válida: {}", blockchain.validar());
        }
        Command::Mine => {
            let mut blockchain = Blockchain::cargar("blockchain.json")
                .unwrap_or_else(|_| Blockchain::new_blockchain());

            if let Ok(wallet) = Wallet::cargar_cifrado("wallet.json", &wallet_password()) {
                blockchain.add_coinbase(wallet.pubkey, 50);
            }

            println!("Minando bloque...");
            blockchain.minar();
            blockchain.guardar("blockchain.json").expect("Error al guardar");
            println!("Bloque minado exitosamente.");
        }
        Command::Send { from, to, amount } => {
            let mut blockchain = Blockchain::cargar("blockchain.json")
                .unwrap_or_else(|_| Blockchain::new_blockchain());

            let from_bytes: [u8; 32] = hex::decode(&from)
                .expect("Clave 'from' inválida")
                .try_into()
                .expect("'from' debe ser 32 bytes");

            let to_bytes: [u8; 32] = hex::decode(&to)
                .expect("Clave 'to' inválida")
                .try_into()
                .expect("'to' debe ser 32 bytes");

            let mut tx = Transaction::new(from_bytes, to_bytes, amount);

            let wallet = Wallet::cargar_cifrado("wallet.json", &wallet_password())
                .expect("Error al cargar la wallet");
            let signing_key = SigningKey::from_bytes(&wallet.secret);
            tx.firmar(&signing_key);

            println!("Enviando {} desde {} a {}...", amount, from, to);
            blockchain.add_transaction(tx).expect("Saldo insuficiente");
            blockchain.guardar("blockchain.json").expect("Error al guardar");
            println!("Transacción en mempool. Usá 'mine' para confirmarla.");
        }
    }
}