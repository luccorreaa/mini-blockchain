use thiserror::Error;

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("wrong password or corrupted wallet file")]
    WrongPassword,
    #[error("unexpected key length in decrypted payload")]
    DecryptedKeyLength,
    #[error("invalid mnemonic: {0}")]
    InvalidMnemonic(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum TransactionError {
    #[error("invalid signature")]
    InvalidSignature,
    #[error("invalid public key")]
    InvalidPublicKey,
    #[error("invalid key length (expected 32 bytes)")]
    InvalidKeyLength,
    #[error("invalid signature length (expected 64 bytes)")]
    InvalidSignatureLength,
}

#[derive(Debug, Error)]
pub enum ChainError {
    #[error("insufficient balance: available {available}, required {required}")]
    InsufficientBalance { available: u64, required: u64 },
    #[error(transparent)]
    Transaction(#[from] TransactionError),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("serialization error: {0}")]
    Serialization(#[from] serde_json::Error),
}

#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid hex: {0}")]
    InvalidHex(#[from] hex::FromHexError),
    #[error("key must be 32 bytes")]
    InvalidKeyLength,
    #[error("wallet already exists at {0}; remove it first to generate a new one")]
    WalletAlreadyExists(String),
    #[error(transparent)]
    Chain(#[from] ChainError),
    #[error(transparent)]
    Wallet(#[from] WalletError),
}

#[derive(Debug, Error)]
pub enum NodeError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error(transparent)]
    Chain(#[from] ChainError),
}

pub type WalletResult<T> = Result<T, WalletError>;
pub type ChainResult<T>  = Result<T, ChainError>;
pub type CliResult<T>    = Result<T, CliError>;
pub type NodeResult<T>   = Result<T, NodeError>;
