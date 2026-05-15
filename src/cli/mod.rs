pub mod parser;
pub mod commands;

use parser::Command;
use crate::config::Config;
use crate::error::CliResult;

pub fn run(command: Command, config: &Config) -> CliResult<()> {
    match command {
        Command::NewWallet                 => commands::new_wallet(config),
        Command::ShowChain                 => commands::show_chain(config),
        Command::Validate                  => commands::validate_chain(config),
        Command::Mine                      => commands::mine(config),
        Command::Send { from, to, amount } => commands::send(&from, &to, amount, config),
    }
}
