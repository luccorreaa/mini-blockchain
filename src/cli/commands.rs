use crate::chain::blockchain::Blockchain;
use crate::crypto::wallet::{self, Wallet};
use crate::crypto::transaction::Transaction;
use crate::types::PublicKey;
use crate::config::Config;
use crate::error::{CliError, CliResult, ChainError};

pub fn new_wallet(config: &Config) -> CliResult<()> {
    if config.wallet_path.exists() {
        return Err(CliError::WalletAlreadyExists(
            config.wallet_path.display().to_string(),
        ));
    }
    let (mnemonic, w) = Wallet::generate()?;
    w.save_encrypted(&config.wallet_path, &config.wallet_password)?;

    println!("\nWallet generated successfully.");
    println!("\nSeed phrase (write this down — it is the only way to recover your wallet):\n");
    println!("  {}\n", mnemonic);
    println!("Public key: {}", w.pubkey());
    println!("\nWARNING: never share your seed phrase with anyone.\n");
    Ok(())
}

pub fn import_wallet(mnemonic: &str, config: &Config) -> CliResult<()> {
    let w = Wallet::from_mnemonic(mnemonic)?;
    w.save_encrypted(&config.wallet_path, &config.wallet_password)?;
    println!("Wallet imported successfully.");
    println!("Public key: {}", w.pubkey());
    Ok(())
}

pub fn show_chain(config: &Config) -> CliResult<()> {
    let blockchain = Blockchain::load(&config.chain_path).unwrap_or_default();
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
    let blockchain = Blockchain::load(&config.chain_path).unwrap_or_default();
    println!("Chain is valid: {}", blockchain.validate());
    Ok(())
}

pub fn mine(config: &Config) -> CliResult<()> {
    let mut blockchain = Blockchain::load(&config.chain_path).unwrap_or_default();
    if let Ok(w) = Wallet::load_encrypted(&config.wallet_path, &config.wallet_password) {
        blockchain.add_coinbase(w.pubkey(), config.coinbase_reward);
    }
    println!("Mining block...");
    blockchain.mine();
    blockchain.save(&config.chain_path)?;
    println!("Block mined successfully.");
    Ok(())
}

pub fn send(mnemonic: &str, to: &str, amount: u64, config: &Config) -> CliResult<()> {
    let signing_key = wallet::signing_key_from_mnemonic(mnemonic)
        .map_err(CliError::Wallet)?;
    let from = PublicKey::from_bytes(signing_key.verifying_key().to_bytes());
    let to_key = PublicKey::try_from(hex::decode(to)?)
        .map_err(|e| CliError::Chain(ChainError::Transaction(e)))?;
    let mut blockchain = Blockchain::load(&config.chain_path).unwrap_or_default();
    let mut tx = Transaction::new(from, to_key, amount);
    tx.sign(&signing_key);
    println!("Sending {} from {} to {}...", amount, from, to_key);
    blockchain.add_transaction(tx)?;
    blockchain.save(&config.chain_path)?;
    println!("Transaction added to mempool. Run 'mine' to confirm it.");
    Ok(())
}
