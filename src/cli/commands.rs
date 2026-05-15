use crate::chain::blockchain::Blockchain;
use crate::crypto::wallet::{self, Wallet};
use crate::crypto::transaction::Transaction;
use crate::types::PublicKey;
use crate::config::Config;
use crate::error::{CliError, CliResult, ChainError, ChainResult, WalletError};

fn load_blockchain(config: &Config) -> ChainResult<Blockchain> {
    match Blockchain::load(&config.chain_path) {
        Ok(bc) => Ok(bc),
        Err(ChainError::Io(ref e)) if e.kind() == std::io::ErrorKind::NotFound => Ok(Blockchain::default()),
        Err(e) => Err(e),
    }
}

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
    let blockchain = load_blockchain(config)?;
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
    let blockchain = load_blockchain(config)?;
    println!("Chain is valid: {}", blockchain.validate());
    Ok(())
}

pub fn mine(config: &Config) -> CliResult<()> {
    let mut blockchain = load_blockchain(config)?;
    match Wallet::load_encrypted(&config.wallet_path, &config.wallet_password) {
        Ok(w) => { blockchain.add_coinbase(w.pubkey(), config.coinbase_reward); }
        Err(WalletError::Io(_)) => {}
        Err(e) => eprintln!("Warning: could not load wallet for coinbase: {e}"),
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
        .map_err(ChainError::Transaction)?;
    let mut blockchain = load_blockchain(config)?;
    let mut tx = Transaction::new(from, to_key, amount);
    tx.sign(&signing_key);
    blockchain.add_transaction(tx)?;
    blockchain.save(&config.chain_path)?;
    println!("Transaction queued: {} units from {} to {}. Run 'mine' to confirm.", amount, from, to_key);
    Ok(())
}
