use std::fmt;
use serde::{Serialize, Deserialize};
use crate::error::TransactionError;

/// SHA-256 hash of a block, stored as a lowercase hex string.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(transparent)]
pub struct Hash(String);

impl Hash {
    pub fn empty() -> Self { Self(String::new()) }
    pub fn as_str(&self) -> &str { &self.0 }
}

impl fmt::Display for Hash {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result { self.0.fmt(f) }
}

impl From<String> for Hash {
    fn from(s: String) -> Self { Self(s) }
}

/// Ed25519 public key (32 bytes), serialized as a hex string.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(transparent)]
pub struct PublicKey(#[serde(with = "hex")] pub(crate) [u8; 32]);

impl PublicKey {
    /// Coinbase sentinel: all-zero bytes indicate no real sender.
    pub fn coinbase() -> Self { Self([0u8; 32]) }
    pub fn from_bytes(bytes: [u8; 32]) -> Self { Self(bytes) }
    pub fn is_coinbase(&self) -> bool { self.0 == [0u8; 32] }
    pub fn as_bytes(&self) -> &[u8; 32] { &self.0 }
}

impl fmt::Display for PublicKey {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", hex::encode(self.0))
    }
}

impl TryFrom<Vec<u8>> for PublicKey {
    type Error = TransactionError;
    fn try_from(v: Vec<u8>) -> Result<Self, Self::Error> {
        let arr: [u8; 32] = v.try_into().map_err(|_| TransactionError::InvalidKeyLength)?;
        Ok(Self(arr))
    }
}
