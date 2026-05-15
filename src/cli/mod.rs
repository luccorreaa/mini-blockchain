use crate::cli::parser::Command;
use crate::config::Config;
use crate::error::CliResult;

pub mod commands;
pub mod parser;

pub fn run(command: Command, config: &Config) -> CliResult<()> {
    match command {
        Command::NewWallet                       => commands::new_wallet(config),
        Command::ImportWallet { mnemonic }       => commands::import_wallet(&mnemonic, config),
        Command::ShowChain                       => commands::show_chain(config),
        Command::Validate                        => commands::validate_chain(config),
        Command::Mine                            => commands::mine(config),
        Command::Send { mnemonic, to, amount }   => commands::send(&mnemonic, &to, amount, config),
    }
}
