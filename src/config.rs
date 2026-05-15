use std::path::PathBuf;

pub const DEFAULT_WALLET_PATH: &str = "wallet.json";
pub const DEFAULT_CHAIN_PATH:  &str = "blockchain.json";
pub const DEFAULT_DIFFICULTY:  usize = 2;
pub const COINBASE_REWARD:     u64   = 50;
pub const API_BIND_ADDR:       &str  = "0.0.0.0:3000";

#[derive(Clone, Debug)]
pub struct Config {
    pub wallet_path:     PathBuf,
    pub chain_path:      PathBuf,
    pub difficulty:      usize,
    pub coinbase_reward: u64,
    pub wallet_password: String,
}

impl Config {
    pub fn from_env() -> Self {
        Self {
            wallet_path:     PathBuf::from(DEFAULT_WALLET_PATH),
            chain_path:      PathBuf::from(DEFAULT_CHAIN_PATH),
            difficulty:      DEFAULT_DIFFICULTY,
            coinbase_reward: COINBASE_REWARD,
            wallet_password: std::env::var("WALLET_PASSWORD")
                .unwrap_or_else(|_| "dev_password_change_me".to_string()),
        }
    }
}

impl Default for Config {
    fn default() -> Self { Self::from_env() }
}
