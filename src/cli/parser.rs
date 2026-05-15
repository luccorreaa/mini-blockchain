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
    /// Generate a new Ed25519 wallet and display the 12-word seed phrase.
    NewWallet,
    /// Reconstruct a wallet from an existing BIP-39 seed phrase.
    ImportWallet {
        /// The 12-word seed phrase (quote the whole phrase).
        #[arg(short, long)]
        mnemonic: String,
    },
    /// Print every block in the chain.
    ShowChain,
    /// Validate the chain and report the result.
    Validate,
    /// Mine the next block from the current mempool.
    Mine,
    /// Create a signed transaction and add it to the mempool.
    Send {
        /// 12-word BIP-39 seed phrase of the sender wallet.
        #[arg(short, long)]
        mnemonic: String,
        /// Recipient public key (hex).
        #[arg(short, long)]
        to: String,
        #[arg(short, long)]
        amount: u64,
    },
}
