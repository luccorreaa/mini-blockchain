use std::path::PathBuf;
use clap::{Parser, Subcommand};
use crate::config::{DEFAULT_WALLET_PATH, DEFAULT_CHAIN_PATH};

#[derive(Parser)]
#[command(name = "Mini Blockchain")]
pub struct Cli {
    #[arg(long, default_value = DEFAULT_WALLET_PATH)]
    pub wallet: PathBuf,
    #[arg(long, default_value = DEFAULT_CHAIN_PATH)]
    pub chain: PathBuf,
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    /// Generate a new Ed25519 wallet.
    NewWallet,
    /// Print every block in the chain.
    ShowChain,
    /// Validate the chain and report the result.
    Validate,
    /// Mine the next block from the current mempool.
    Mine,
    /// Create a signed transaction and add it to the mempool.
    Send {
        #[arg(short, long)]
        from: String,
        #[arg(short, long)]
        to: String,
        #[arg(short, long)]
        amount: u64,
    },
}
