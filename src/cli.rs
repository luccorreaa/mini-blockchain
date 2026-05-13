//! Command-line interface definition for the mini-blockchain node.
//!
//! Uses [`clap`] derive macros to parse subcommands. The entry point in
//! `main.rs` matches on [`Command`] variants to dispatch each operation.

use clap::{Parser, Subcommand};

/// Root CLI struct. Parse with [`Cli::parse`].
#[derive(Parser)]
#[command(name = "Mini Blockchain")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

/// Available CLI subcommands.
#[derive(Subcommand)]
pub enum Command {
    /// Generate a new Ed25519 wallet and save it encrypted to `wallet.json`.
    NewWallet,
    /// Print every block in the chain loaded from `blockchain.json`.
    ShowChain,
    /// Validate the chain loaded from `blockchain.json` and report the result.
    Validate,
    /// Mine the next block from the current mempool and append it to the chain.
    Mine,
    /// Create a signed transaction and add it to the mempool.
    Send {
        /// Sender's public key (hex-encoded, 32 bytes).
        #[arg(short, long)]
        from: String,
        /// Recipient's public key (hex-encoded, 32 bytes).
        #[arg(short, long)]
        to: String,
        /// Amount of units to transfer.
        #[arg(short, long)]
        amount: u64,
    },
}
