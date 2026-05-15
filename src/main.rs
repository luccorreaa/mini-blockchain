mod cli;
use crate::cli::{Cli, Command};
use clap::Parser;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
use mini_blockchain::blockchain::Blockchain;
use mini_blockchain::crypto::wallet::Wallet;
use mini_blockchain::crypto::transaction::Transaction;
use mini_blockchain::types::PublicKey;

/// Returns the wallet encryption password from the `WALLET_PASSWORD` environment variable,
/// falling back to an insecure development default when the variable is not set.
fn wallet_password() -> String {
    std::env::var("WALLET_PASSWORD").unwrap_or_else(|_| "dev_password_change_me".to_string())
}

fn main() {
    let cli = Cli::parse();
    match cli.command {
        Command::NewWallet => {
            if std::path::Path::new("wallet.json").exists() {
                eprintln!("A wallet already exists. Remove wallet.json before generating a new one.");
                std::process::exit(1);
            }
            let mut secret = [0u8; 32];
            OsRng.fill_bytes(&mut secret);
            let signing_key = SigningKey::from_bytes(&secret);
            let pubkey = PublicKey(signing_key.verifying_key().to_bytes());
            let wallet = Wallet::new(secret, pubkey);
            wallet.save_encrypted("wallet.json", &wallet_password())
                .expect("Failed to save wallet");
            println!("Generating new wallet...");
            println!("Public key: {}", pubkey);
        }
        Command::ShowChain => {
            let blockchain = Blockchain::load("blockchain.json")
                .unwrap_or_else(|_| Blockchain::new());
            println!("Showing blockchain...");
            for block in blockchain.chain() {
                println!(
                    "Block {}: Hash: {}, Prev Hash: {}, Timestamp: {}, Transactions: {}",
                    block.index(), block.hash(), block.prev_hash(),
                    block.timestamp(), block.transactions().len()
                );
            }
        }
        Command::Validate => {
            let blockchain = Blockchain::load("blockchain.json")
                .unwrap_or_else(|_| Blockchain::new());
            println!("Validating blockchain...");
            println!("Chain is valid: {}", blockchain.validate());
        }
        Command::Mine => {
            let mut blockchain = Blockchain::load("blockchain.json")
                .unwrap_or_else(|_| Blockchain::new());

            if let Ok(wallet) = Wallet::load_encrypted("wallet.json", &wallet_password()) {
                blockchain.add_coinbase(wallet.pubkey, 50);
            }

            println!("Mining block...");
            blockchain.mine();
            blockchain.save("blockchain.json").expect("Failed to save blockchain");
            println!("Block mined successfully.");
        }
        Command::Send { from, to, amount } => {
            let mut blockchain = Blockchain::load("blockchain.json")
                .unwrap_or_else(|_| Blockchain::new());

            let from_bytes: [u8; 32] = hex::decode(&from)
                .expect("Invalid 'from' key")
                .try_into()
                .expect("'from' must be 32 bytes");

            let to_bytes: [u8; 32] = hex::decode(&to)
                .expect("Invalid 'to' key")
                .try_into()
                .expect("'to' must be 32 bytes");

            let mut tx = Transaction::new(PublicKey(from_bytes), PublicKey(to_bytes), amount);
            let wallet = Wallet::load_encrypted("wallet.json", &wallet_password())
                .expect("Failed to load wallet");
            let signing_key = SigningKey::from_bytes(&wallet.secret);
            tx.sign(&signing_key);

            println!("Sending {} from {} to {}...", amount, from, to);
            blockchain.add_transaction(tx).expect("Insufficient balance");
            blockchain.save("blockchain.json").expect("Failed to save blockchain");
            println!("Transaction in mempool. Run 'mine' to confirm it.");
        }
    }
}
