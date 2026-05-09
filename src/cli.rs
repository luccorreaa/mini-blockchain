//cli.rs
use clap::{Parser, Subcommand};
#[derive(Parser)]
#[command(name = "Mini Blockchain")]
pub struct Cli {
    #[command(subcommand)]
    pub command: Command,
}

#[derive(Subcommand)]
pub enum Command {
    NewWallet,
    ShowChain,
    Validate,
    Send {
        #[arg(short, long)]
        from: String,
        #[arg(short, long)]
        to: String,
        #[arg(short, long)]
        amount: u64,
    }
}

