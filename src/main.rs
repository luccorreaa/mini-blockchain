use clap::Parser;
use mini_blockchain::cli::parser::Cli;
use mini_blockchain::config::Config;

fn main() {
    let cli = Cli::parse();
    let config = Config {
        wallet_path: cli.wallet.clone(),
        chain_path:  cli.chain.clone(),
        ..Config::from_env()
    };
    if let Err(e) = mini_blockchain::cli::run(cli.command, &config) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
