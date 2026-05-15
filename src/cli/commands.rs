use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
use crate::chain::blockchain::Blockchain;
use crate::crypto::wallet::Wallet;
use crate::crypto::transaction::Transaction;
use crate::types::PublicKey;
use crate::config::Config;
use crate::error::{CliError, CliResult, ChainError};

pub fn new_wallet(config: &Config) -> CliResult<()> {
    if config.wallet_path.exists() {
        eprintln!("A wallet already exists. Remove {:?} before generating a new one.", config.wallet_path);
        std::process::exit(1);
    }
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let signing_key = SigningKey::from_bytes(&secret);
    let pubkey = PublicKey(signing_key.verifying_key().to_bytes());
    let wallet = Wallet::new(secret, pubkey);
    wallet.save_encrypted(config.wallet_path.to_str().unwrap(), &config.wallet_password)?;
    println!("Generating new wallet...");
    println!("Public key: {}", pubkey);
    Ok(())
}

pub fn show_chain(config: &Config) -> CliResult<()> {
    let blockchain = Blockchain::load(config.chain_path.to_str().unwrap())
        .unwrap_or_default();
    println!("Showing blockchain...");
    for block in blockchain.chain() {
        println!(
            "Block {}: Hash: {}, Prev Hash: {}, Timestamp: {}, Transactions: {}",
            block.index(), block.hash(), block.prev_hash(),
            block.timestamp(), block.transactions().len()
        );
    }
    Ok(())
}

pub fn validate_chain(config: &Config) -> CliResult<()> {
    let blockchain = Blockchain::load(config.chain_path.to_str().unwrap())
        .unwrap_or_default();
    println!("Validating blockchain...");
    println!("Chain is valid: {}", blockchain.validate());
    Ok(())
}

pub fn mine(config: &Config) -> CliResult<()> {
    let chain_path = config.chain_path.to_str().unwrap();
    let mut blockchain = Blockchain::load(chain_path).unwrap_or_default();
    if let Ok(wallet) = Wallet::load_encrypted(
        config.wallet_path.to_str().unwrap(),
        &config.wallet_password,
    ) {
        blockchain.add_coinbase(wallet.pubkey, config.coinbase_reward);
    }
    println!("Mining block...");
    blockchain.mine();
    blockchain.save(chain_path)?;
    println!("Block mined successfully.");
    Ok(())
}

pub fn send(from: &str, to: &str, amount: u64, config: &Config) -> CliResult<()> {
    let chain_path = config.chain_path.to_str().unwrap();
    let mut blockchain = Blockchain::load(chain_path).unwrap_or_default();
    let from_key = PublicKey::try_from(hex::decode(from)?)
        .map_err(|e| CliError::Chain(ChainError::Transaction(e)))?;
    let to_key = PublicKey::try_from(hex::decode(to)?)
        .map_err(|e| CliError::Chain(ChainError::Transaction(e)))?;
    let mut tx = Transaction::new(from_key, to_key, amount);
    let wallet = Wallet::load_encrypted(
        config.wallet_path.to_str().unwrap(),
        &config.wallet_password,
    )?;
    tx.sign(&SigningKey::from_bytes(&wallet.secret));
    println!("Sending {} from {} to {}...", amount, from_key, to_key);
    blockchain.add_transaction(tx)?;
    blockchain.save(chain_path)?;
    println!("Transaction in mempool. Run 'mine' to confirm it.");
    Ok(())
}
