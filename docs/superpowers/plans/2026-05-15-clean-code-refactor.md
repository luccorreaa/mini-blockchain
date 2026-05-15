# Clean Code Refactor — Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Refactor mini_blockchain into a clean, idiomatic Rust codebase with typed errors, semantic newtypes, a Config struct, proper module hierarchy, and thin binaries.

**Architecture:** Foundation types (error.rs, types.rs, config.rs) are added first since everything depends on them. Then crypto/ and chain/ modules replace the flat root files. Finally CLI, API, and node binaries are restructured so each binary is ≤10 lines.

**Tech Stack:** Rust 2024, thiserror 1.0, ed25519-dalek 2, axum 0.7, libp2p 0.54, clap 4 (derive), serde/serde_json, aes-gcm, tokio.

---

## File Map

### Created
- `src/error.rs` — WalletError, TransactionError, ChainError, CliError, NodeError + Result aliases
- `src/types.rs` — Hash, PublicKey newtypes
- `src/config.rs` — Config struct + DEFAULT_* constants
- `src/crypto/mod.rs` — re-exports transaction and wallet
- `src/crypto/transaction.rs` — Transaction (private fields + getters, PublicKey sender/receiver)
- `src/crypto/wallet.rs` — Wallet (WalletError return types)
- `src/chain/mod.rs` — re-exports block, blockchain, merkle
- `src/chain/merkle.rs` — merkle_root with sha256_hex helper
- `src/chain/block.rs` — Block using Hash/PublicKey
- `src/chain/blockchain.rs` — Blockchain with split validate() and ChainError
- `src/cli/mod.rs` — run() dispatcher
- `src/cli/parser.rs` — Cli, Command with global --wallet/--chain flags
- `src/cli/commands.rs` — one fn per subcommand
- `src/api/mod.rs` — AppState, build_router(), serve()
- `src/api/handlers/mod.rs` — re-exports
- `src/api/handlers/chain.rs` — get_chain, get_block, validate handlers
- `src/api/handlers/mining.rs` — mine handler (3-phase lock-free PoW)
- `src/api/handlers/transaction.rs` — add_to_mempool handler
- `src/api/handlers/wallet.rs` — new_wallet handler

- `src/node/mod.rs` — Node struct with new() + run()
- `src/node/behaviour.rs` — NodeBehaviour, ChainRequest, ChainResponse
- `src/node/events.rs` — handle_swarm_event()
- `src/node/commands.rs` — handle_stdin_command()

### Modified
- `Cargo.toml` — add thiserror
- `src/lib.rs` — updated module re-exports
- `src/main.rs` — thin CLI entry point
- `src/bin/api.rs` — thin API entry point
- `src/bin/node.rs` — thin node entry point

### Deleted
- `src/transactions.rs`
- `src/wallet.rs`
- `src/block.rs`
- `src/blockchain.rs`
- `src/merkle.rs`
- `src/cli.rs`

---

## Task 1: Add thiserror + create src/error.rs

**Files:**
- Modify: `Cargo.toml`
- Create: `src/error.rs`

- [ ] **Step 1: Add thiserror to Cargo.toml**

In `[dependencies]`, add:
```toml
thiserror = "1.0"
```

- [ ] **Step 2: Create src/error.rs**

```rust
use thiserror::Error;

#[derive(Debug, Error)]
pub enum WalletError {
    #[error("encryption failed: {0}")]
    EncryptionFailed(String),
    #[error("wrong password or corrupted wallet file")]
    WrongPassword,
    #[error("unexpected key length in decrypted payload")]
    DecryptedKeyLength,
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
```

- [ ] **Step 3: Add to src/lib.rs (append at top)**

Add `pub mod error;` to lib.rs.

- [ ] **Step 4: Verify it compiles**

```bash
cargo build 2>&1 | head -20
```
Expected: compiles without errors.

- [ ] **Step 5: Commit**

```bash
git add Cargo.toml Cargo.lock src/error.rs src/lib.rs
git commit -m "feat: add thiserror and error enums"
```

---

## Task 2: Create src/types.rs

**Files:**
- Create: `src/types.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create src/types.rs**

```rust
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
pub struct PublicKey(#[serde(with = "hex")] pub [u8; 32]);

impl PublicKey {
    /// Coinbase sentinel: all-zero bytes indicate no real sender.
    pub fn coinbase() -> Self { Self([0u8; 32]) }
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
```

- [ ] **Step 2: Add to src/lib.rs**

Add `pub mod types;` to lib.rs.

- [ ] **Step 3: Verify it compiles**

```bash
cargo build 2>&1 | head -20
```
Expected: no errors.

- [ ] **Step 4: Commit**

```bash
git add src/types.rs src/lib.rs
git commit -m "feat: add Hash and PublicKey semantic newtypes"
```

---

## Task 3: Create src/config.rs

**Files:**
- Create: `src/config.rs`
- Modify: `src/lib.rs`

- [ ] **Step 1: Create src/config.rs**

```rust
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
```

- [ ] **Step 2: Add to src/lib.rs**

Add `pub mod config;` to lib.rs.

- [ ] **Step 3: Verify it compiles**

```bash
cargo build 2>&1 | head -20
```

- [ ] **Step 4: Commit**

```bash
git add src/config.rs src/lib.rs
git commit -m "feat: add Config struct and DEFAULT_* constants"
```

---

## Task 4: Create src/crypto/ (replaces transactions.rs + wallet.rs)

**Files:**
- Create: `src/crypto/mod.rs`
- Create: `src/crypto/transaction.rs`
- Create: `src/crypto/wallet.rs`
- Modify: `src/lib.rs`
- Delete: `src/transactions.rs`, `src/wallet.rs`

- [ ] **Step 1: Create src/crypto/mod.rs**

```rust
pub mod transaction;
pub mod wallet;
```

- [ ] **Step 2: Create src/crypto/transaction.rs**

```rust
//! Transaction primitives for the mini-blockchain.

use ed25519_dalek::{SigningKey, Signer};
use rand::random;
use serde::{Serialize, Deserialize};
use crate::types::PublicKey;

/// A transfer of `amount` units from `sender` to `receiver`.
///
/// The `nonce` is randomised at construction to prevent replay attacks.
/// Call [`Transaction::sign`] before submitting to the mempool.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Transaction {
    sender: PublicKey,
    receiver: PublicKey,
    amount: u64,
    nonce: u64,
    signature: Option<Vec<u8>>,
}

impl Transaction {
    pub fn new(sender: PublicKey, receiver: PublicKey, amount: u64) -> Self {
        Self {
            sender,
            receiver,
            amount,
            nonce: random::<u64>(),
            signature: None,
        }
    }

    pub fn sign(&mut self, signing_key: &SigningKey) {
        let content = format!(
            "{}{}{}{}",
            hex::encode(self.sender.as_bytes()),
            hex::encode(self.receiver.as_bytes()),
            self.amount,
            self.nonce
        );
        self.signature = Some(signing_key.sign(content.as_bytes()).to_bytes().to_vec());
    }

    pub fn sender(&self) -> PublicKey { self.sender }
    pub fn receiver(&self) -> PublicKey { self.receiver }
    pub fn amount(&self) -> u64 { self.amount }
    pub fn nonce(&self) -> u64 { self.nonce }
    pub fn signature(&self) -> Option<&[u8]> { self.signature.as_deref() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transacciones_identicas_tienen_nonce_distinto() {
        let tx1 = Transaction::new(PublicKey::coinbase(), PublicKey([1u8; 32]), 100);
        let tx2 = Transaction::new(PublicKey::coinbase(), PublicKey([1u8; 32]), 100);
        assert_ne!(tx1.nonce(), tx2.nonce());
    }
}
```

- [ ] **Step 3: Create src/crypto/wallet.rs**

```rust
//! Wallet: Ed25519 key pair with AES-256-GCM encrypted persistence.

use aes_gcm::{Aes256Gcm, Key, Nonce};
use aes_gcm::aead::{Aead, KeyInit};
use aes_gcm::aead::rand_core::RngCore;
use aes_gcm::aead::OsRng;
use serde::{Serialize, Deserialize};
use sha2::{Sha256, Digest};
use crate::types::PublicKey;
use crate::error::{WalletError, WalletResult};

/// An Ed25519 key pair used for signing transactions and blocks.
#[derive(Serialize, Deserialize)]
pub struct Wallet {
    /// Raw 32-byte Ed25519 secret (signing) key, hex-encoded in JSON.
    #[serde(with = "hex")]
    pub secret: [u8; 32],
    /// Corresponding Ed25519 public (verifying) key.
    pub pubkey: PublicKey,
}

#[derive(Serialize, Deserialize)]
struct EncryptedWallet {
    #[serde(with = "hex")]
    nonce: [u8; 12],
    #[serde(with = "hex")]
    ciphertext: Vec<u8>,
    pubkey: PublicKey,
}

impl Wallet {
    pub fn new(secret: [u8; 32], pubkey: PublicKey) -> Self {
        Self { secret, pubkey }
    }

    fn derive_key(password: &str) -> [u8; 32] {
        let mut hasher = Sha256::new();
        hasher.update(password.as_bytes());
        hasher.finalize().into()
    }

    pub fn save_encrypted(&self, path: &str, password: &str) -> WalletResult<()> {
        let key_bytes = Self::derive_key(password);
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        let mut nonce_bytes = [0u8; 12];
        OsRng.fill_bytes(&mut nonce_bytes);
        let nonce = Nonce::from_slice(&nonce_bytes);
        let ciphertext = cipher.encrypt(nonce, self.secret.as_ref())
            .map_err(|e| WalletError::EncryptionFailed(e.to_string()))?;
        let encrypted = EncryptedWallet { nonce: nonce_bytes, ciphertext, pubkey: self.pubkey };
        std::fs::write(path, serde_json::to_string_pretty(&encrypted)?)?;
        Ok(())
    }

    pub fn load_encrypted(path: &str, password: &str) -> WalletResult<Wallet> {
        let encrypted: EncryptedWallet = serde_json::from_str(&std::fs::read_to_string(path)?)?;
        let key_bytes = Self::derive_key(password);
        let key = Key::<Aes256Gcm>::from_slice(&key_bytes);
        let cipher = Aes256Gcm::new(key);
        let nonce = Nonce::from_slice(&encrypted.nonce);
        let plaintext = cipher.decrypt(nonce, encrypted.ciphertext.as_ref())
            .map_err(|_| WalletError::WrongPassword)?;
        let secret: [u8; 32] = plaintext.try_into()
            .map_err(|_| WalletError::DecryptedKeyLength)?;
        Ok(Wallet { secret, pubkey: encrypted.pubkey })
    }

    pub fn save(&self, path: &str) -> WalletResult<()> {
        std::fs::write(path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }

    pub fn load(path: &str) -> WalletResult<Wallet> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cifrado_round_trip() {
        let wallet = Wallet::new([42u8; 32], PublicKey([7u8; 32]));
        wallet.save_encrypted("/tmp/test_wallet_rt.json", "mi_password").unwrap();
        let loaded = Wallet::load_encrypted("/tmp/test_wallet_rt.json", "mi_password").unwrap();
        assert_eq!(loaded.secret, wallet.secret);
        assert_eq!(loaded.pubkey, wallet.pubkey);
    }

    #[test]
    fn password_incorrecta_falla() {
        let wallet = Wallet::new([1u8; 32], PublicKey([2u8; 32]));
        wallet.save_encrypted("/tmp/test_wallet_pw.json", "correcta").unwrap();
        assert!(Wallet::load_encrypted("/tmp/test_wallet_pw.json", "incorrecta").is_err());
    }
}
```

- [ ] **Step 4: Update src/lib.rs**

Replace `pub mod transactions;` and `pub mod wallet;` with:
```rust
pub mod crypto;
```

Remove the old lines. Keep all other existing `pub mod` lines.

- [ ] **Step 5: Delete old files**

```bash
rm src/transactions.rs src/wallet.rs
```

- [ ] **Step 6: Fix any compilation errors from consumers still importing old paths**

The only consumers at this point are `src/block.rs`, `src/blockchain.rs`, `src/merkle.rs`, `src/main.rs`, `src/bin/api.rs`, `src/bin/node.rs`. Update their imports:

- `use crate::transactions::Transaction;` → `use crate::crypto::transaction::Transaction;`
- `use mini_blockchain::transactions::Transaction;` → `use mini_blockchain::crypto::transaction::Transaction;`
- `use mini_blockchain::wallet::Wallet;` → `use mini_blockchain::crypto::wallet::Wallet;`
- `use crate::wallet::Wallet;` (if any) → same pattern

Also update any `Transaction::new([0u8; 32], ...)` calls — these are not yet changed since `block.rs` etc. haven't been migrated. Temporarily add `use mini_blockchain::types::PublicKey;` where needed and change `[0u8; 32]` → `PublicKey::coinbase()` and `[Xu8; 32]` → `PublicKey([Xu8; 32])` for non-coinbase keys.

- [ ] **Step 7: Run tests**

```bash
cargo test 2>&1
```
Expected: all tests pass.

- [ ] **Step 8: Commit**

```bash
git add src/crypto/ src/lib.rs
git rm src/transactions.rs src/wallet.rs
git commit -m "refactor: move Transaction and Wallet into crypto/ module"
```

---

## Task 5: Create src/chain/ (replaces merkle.rs + block.rs + blockchain.rs)

**Files:**
- Create: `src/chain/mod.rs`
- Create: `src/chain/merkle.rs`
- Create: `src/chain/block.rs`
- Create: `src/chain/blockchain.rs`
- Modify: `src/lib.rs`
- Delete: `src/merkle.rs`, `src/block.rs`, `src/blockchain.rs`

- [ ] **Step 1: Create src/chain/mod.rs**

```rust
pub mod block;
pub mod blockchain;
pub mod merkle;
```

- [ ] **Step 2: Create src/chain/merkle.rs**

```rust
//! Merkle tree root computation for transaction sets.

use sha2::{Sha256, Digest};
use crate::crypto::transaction::Transaction;
use crate::types::Hash;

fn sha256_hex(content: &str) -> Hash {
    let mut h = Sha256::new();
    h.update(content.as_bytes());
    Hash::from(format!("{:x}", h.finalize()))
}

/// Computes the SHA-256 Merkle root of `transactions`.
///
/// Returns an empty Hash when `transactions` is empty.
/// Odd-length layers duplicate the last hash before combining pairs.
pub fn merkle_root(transactions: &[Transaction]) -> Hash {
    if transactions.is_empty() {
        return Hash::empty();
    }
    let mut hashes: Vec<Hash> = transactions.iter().map(|tx| {
        sha256_hex(&format!(
            "{}{}{}{}",
            hex::encode(tx.sender().as_bytes()),
            hex::encode(tx.receiver().as_bytes()),
            tx.amount(),
            tx.nonce()
        ))
    }).collect();

    while hashes.len() > 1 {
        hashes = hashes.chunks(2).map(|pair| {
            let left = &pair[0];
            let right = pair.get(1).unwrap_or(left);
            sha256_hex(&format!("{}{}", left, right))
        }).collect();
    }
    hashes.remove(0)
}
```

- [ ] **Step 3: Create src/chain/block.rs**

```rust
//! Block structure and Proof-of-Work mining.

use ed25519_dalek::{SigningKey, Signer};
use sha2::{Sha256, Digest};
use std::time::SystemTime;
use serde::{Serialize, Deserialize};
use crate::crypto::transaction::Transaction;
use crate::chain::merkle::merkle_root;
use crate::types::{Hash, PublicKey};

/// A single block in the blockchain.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Block {
    index: u32,
    transactions: Vec<Transaction>,
    prev_hash: Hash,
    hash: Hash,
    timestamp: u64,
    signature: Option<Vec<u8>>,
    author: Option<PublicKey>,
    nonce: u64,
}

impl Block {
    /// Computes the SHA-256 hash of this block's header fields.
    pub fn compute_hash(&self) -> Hash {
        let content = format!(
            "{}{}{}{}{}",
            self.index, merkle_root(&self.transactions), self.prev_hash, self.timestamp, self.nonce
        );
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        Hash::from(format!("{:x}", hasher.finalize()))
    }

    /// Creates a new unmined block. Call [`Block::mine`] before appending to the chain.
    pub fn new(index: u32, transactions: Vec<Transaction>, prev_hash: &Hash) -> Block {
        let mut block = Block {
            index,
            transactions,
            prev_hash: prev_hash.clone(),
            hash: Hash::empty(),
            timestamp: SystemTime::now()
                .duration_since(SystemTime::UNIX_EPOCH)
                .unwrap()
                .as_secs(),
            signature: None,
            author: None,
            nonce: 0,
        };
        block.hash = block.compute_hash();
        block
    }

    /// Signs the block with the given Ed25519 signing key.
    pub fn sign(&mut self, signing_key: &SigningKey) {
        let content = format!(
            "{}{}{}{}",
            self.index, merkle_root(&self.transactions), self.prev_hash, self.timestamp
        );
        let signature = signing_key.sign(content.as_bytes());
        self.signature = Some(signature.to_bytes().to_vec());
        self.author = Some(PublicKey(signing_key.verifying_key().to_bytes()));
    }

    /// Increments nonce until hash has `difficulty` leading zero hex digits.
    pub fn mine(&mut self, difficulty: usize) {
        let target = "0".repeat(difficulty);
        while !self.hash.as_str().starts_with(&target) {
            self.nonce += 1;
            self.hash = self.compute_hash();
        }
    }

    pub fn hash(&self) -> &Hash { &self.hash }
    pub fn prev_hash(&self) -> &Hash { &self.prev_hash }
    pub fn index(&self) -> u32 { self.index }
    pub fn timestamp(&self) -> u64 { self.timestamp }
    pub fn transactions(&self) -> &[Transaction] { &self.transactions }
    pub fn signature(&self) -> Option<&[u8]> { self.signature.as_deref() }
    pub fn author(&self) -> Option<PublicKey> { self.author }

    #[cfg(test)]
    pub fn corrupt(&mut self) { self.hash = Hash::from("corrupt_hash".to_string()); }

    #[cfg(test)]
    pub fn set_signature_test(&mut self, sig: Vec<u8>) { self.signature = Some(sig); }

    #[cfg(test)]
    pub fn set_author_test(&mut self, author: PublicKey) { self.author = Some(author); }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn hash_es_consistente_al_recalcular() {
        let block = Block::new(0, vec![], &Hash::empty());
        assert_eq!(block.hash(), &block.compute_hash());
    }

    #[test]
    fn hash_cambia_al_agregar_transaccion() {
        let mut block = Block::new(0, vec![], &Hash::empty());
        let hash_original = block.compute_hash();
        block.transactions.push(
            Transaction::new(PublicKey::coinbase(), PublicKey([1u8; 32]), 100)
        );
        assert_ne!(hash_original, block.compute_hash());
    }
}
```

- [ ] **Step 4: Create src/chain/blockchain.rs**

```rust
//! Blockchain: an ordered chain of blocks and a pending-transaction mempool.

use ed25519_dalek::{VerifyingKey, Signature, Verifier};
use serde::{Serialize, Deserialize};
use crate::chain::merkle::merkle_root;
use crate::chain::block::Block;
use crate::crypto::transaction::Transaction;
use crate::types::PublicKey;
use crate::error::{TransactionError, ChainError, ChainResult};

/// An append-only chain of [`Block`]s with a pending-transaction mempool.
#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct Blockchain {
    chain: Vec<Block>,
    #[serde(default)]
    mempool: Vec<Transaction>,
    #[serde(default = "default_difficulty")]
    difficulty: usize,
}

fn default_difficulty() -> usize { 2 }

impl Blockchain {
    pub fn new() -> Self { Self::with_difficulty(2) }

    pub fn with_difficulty(difficulty: usize) -> Self {
        use crate::types::Hash;
        let genesis = Block::new(0, vec![], &Hash::empty());
        Self { chain: vec![genesis], mempool: vec![], difficulty }
    }

    pub fn add_block(&mut self, transactions: Vec<Transaction>) {
        if let Some(tip) = self.chain.last() {
            let new_block = Block::new(tip.index() + 1, transactions, tip.hash());
            self.chain.push(new_block);
        }
    }

    pub fn balance_of(&self, pubkey: &PublicKey) -> u64 {
        let mut balance = 0u64;
        for block in &self.chain {
            for tx in block.transactions() {
                if !tx.sender().is_coinbase() && tx.sender() == *pubkey {
                    balance = balance.saturating_sub(tx.amount());
                }
                if tx.receiver() == *pubkey {
                    balance = balance.saturating_add(tx.amount());
                }
            }
        }
        for tx in &self.mempool {
            if !tx.sender().is_coinbase() && tx.sender() == *pubkey {
                balance = balance.saturating_sub(tx.amount());
            }
        }
        balance
    }

    pub fn add_coinbase(&mut self, miner: PublicKey, reward: u64) {
        let coinbase = Transaction::new(PublicKey::coinbase(), miner, reward);
        self.mempool.insert(0, coinbase);
    }

    pub fn add_transaction(&mut self, transaction: Transaction) -> ChainResult<()> {
        if !transaction.sender().is_coinbase() {
            let available = self.balance_of(&transaction.sender());
            if available < transaction.amount() {
                return Err(ChainError::InsufficientBalance {
                    available,
                    required: transaction.amount(),
                });
            }
        }
        self.mempool.push(transaction);
        Ok(())
    }

    pub fn difficulty(&self) -> usize { self.difficulty }

    pub fn tip(&self) -> Option<(u32, crate::types::Hash)> {
        self.chain.last().map(|b| (b.index(), b.hash().clone()))
    }

    pub fn take_mempool(&mut self) -> Vec<Transaction> {
        std::mem::take(&mut self.mempool)
    }

    pub fn push_block(&mut self, block: Block) {
        self.chain.push(block);
    }

    pub fn mine(&mut self) {
        if let Some((tip_index, tip_hash)) = self.tip() {
            let txs = self.take_mempool();
            let mut new_block = Block::new(tip_index + 1, txs, &tip_hash);
            new_block.mine(self.difficulty);
            self.chain.push(new_block);
        }
    }

    /// Validates the entire chain: hash integrity, block linkage, and signatures.
    pub fn validate(&self) -> bool {
        self.chain.iter().enumerate().all(|(i, block)| self.validate_block(i, block))
    }

    fn validate_block(&self, i: usize, block: &Block) -> bool {
        if block.hash() != &block.compute_hash() { return false; }
        if i > 0 && block.prev_hash() != self.chain[i - 1].hash() { return false; }
        for tx in block.transactions() {
            if tx.sender().is_coinbase() { continue; }
            if Self::verify_tx_signature(tx).is_err() { return false; }
        }
        self.verify_block_signature(block).is_ok()
    }

    fn verify_tx_signature(tx: &Transaction) -> Result<(), TransactionError> {
        let sig_bytes = tx.signature().ok_or(TransactionError::InvalidSignature)?;
        let sig_array: [u8; 64] = sig_bytes.try_into()
            .map_err(|_| TransactionError::InvalidSignatureLength)?;
        let signature = Signature::from_bytes(&sig_array);
        let content = format!(
            "{}{}{}{}",
            hex::encode(tx.sender().as_bytes()),
            hex::encode(tx.receiver().as_bytes()),
            tx.amount(),
            tx.nonce()
        );
        let verifying_key = VerifyingKey::from_bytes(tx.sender().as_bytes())
            .map_err(|_| TransactionError::InvalidPublicKey)?;
        verifying_key.verify(content.as_bytes(), &signature)
            .map_err(|_| TransactionError::InvalidSignature)
    }

    fn verify_block_signature(&self, block: &Block) -> Result<(), ChainError> {
        let (sig_bytes, author) = match (block.signature(), block.author()) {
            (Some(s), Some(a)) => (s, a),
            _ => return Ok(()),
        };
        let sig_array: [u8; 64] = sig_bytes.try_into()
            .map_err(|_| TransactionError::InvalidSignatureLength)?;
        let signature = Signature::from_bytes(&sig_array);
        let content = format!(
            "{}{}{}{}",
            block.index(), merkle_root(block.transactions()), block.prev_hash(), block.timestamp()
        );
        let verifying_key = VerifyingKey::from_bytes(author.as_bytes())
            .map_err(|_| TransactionError::InvalidPublicKey)?;
        verifying_key.verify(content.as_bytes(), &signature)
            .map_err(|_| TransactionError::InvalidSignature)?;
        Ok(())
    }

    pub fn sign_block(&mut self, index: usize, signing_key: &ed25519_dalek::SigningKey) {
        if let Some(block) = self.chain.iter_mut().find(|b| b.index() as usize == index) {
            block.sign(signing_key);
        }
    }

    pub fn save(&self, path: &str) -> ChainResult<()> {
        std::fs::write(path, serde_json::to_string_pretty(&self)?)?;
        Ok(())
    }

    pub fn load(path: &str) -> ChainResult<Blockchain> {
        Ok(serde_json::from_str(&std::fs::read_to_string(path)?)?)
    }

    pub fn chain(&self) -> &[Block] { &self.chain }

    #[cfg(test)]
    pub fn corrupt_block(&mut self, index: usize) {
        if let Some(block) = self.chain.iter_mut().find(|b| b.index() as usize == index) {
            block.corrupt();
        }
    }
}

impl Default for Blockchain {
    fn default() -> Self { Self::new() }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn blockchain_almacena_dificultad_configurada() {
        let bc = Blockchain::with_difficulty(4);
        assert_eq!(bc.difficulty(), 4);
    }

    #[test]
    fn add_transaction_rechaza_si_saldo_insuficiente() {
        let mut blockchain = Blockchain::new();
        let tx = Transaction::new(PublicKey([1u8; 32]), PublicKey([2u8; 32]), 100);
        assert!(blockchain.add_transaction(tx).is_err());
    }

    #[test]
    fn add_coinbase_agrega_a_mempool_sin_validar_saldo() {
        let mut blockchain = Blockchain::new();
        blockchain.add_coinbase(PublicKey([3u8; 32]), 50);
        assert_eq!(blockchain.mempool.len(), 1);
    }

    #[test]
    fn balance_aumenta_tras_minar_coinbase() {
        let mut blockchain = Blockchain::new();
        let miner = PublicKey([3u8; 32]);
        blockchain.add_coinbase(miner, 50);
        blockchain.mine();
        assert_eq!(blockchain.balance_of(&miner), 50);
    }

    #[test]
    fn cadena_corrompida_no_es_valida() {
        let mut blockchain = Blockchain::new();
        blockchain.add_block(vec![]);
        blockchain.add_block(vec![]);
        assert!(blockchain.validate());
        blockchain.corrupt_block(1);
        assert!(!blockchain.validate());
    }

    #[test]
    fn validar_no_panic_con_firma_de_longitud_incorrecta() {
        let mut blockchain = Blockchain::new();
        blockchain.add_block(vec![]);
        if let Some(block) = blockchain.chain.iter_mut().find(|b| b.index() == 1) {
            block.set_signature_test(vec![0u8; 10]);
            block.set_author_test(PublicKey([1u8; 32]));
        }
        assert!(!blockchain.validate());
    }
}
```

- [ ] **Step 5: Update src/lib.rs**

Replace `pub mod block;`, `pub mod blockchain;`, `pub mod merkle;` with:
```rust
pub mod chain;
```

- [ ] **Step 6: Delete old files**

```bash
rm src/block.rs src/blockchain.rs src/merkle.rs
```

- [ ] **Step 7: Fix remaining consumers**

Update `src/main.rs` and `src/bin/api.rs` and `src/bin/node.rs` import paths:
- `mini_blockchain::blockchain::Blockchain` → `mini_blockchain::chain::blockchain::Blockchain`
- `mini_blockchain::block::Block` → `mini_blockchain::chain::block::Block`
- `mini_blockchain::transactions::Transaction` → `mini_blockchain::crypto::transaction::Transaction`
- `mini_blockchain::wallet::Wallet` → `mini_blockchain::crypto::wallet::Wallet`

- [ ] **Step 8: Run all tests**

```bash
cargo test 2>&1
```
Expected: all tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/chain/ src/lib.rs
git rm src/block.rs src/blockchain.rs src/merkle.rs
git commit -m "refactor: move Block, Blockchain, and Merkle into chain/ module"
```

---

## Task 6: Create src/cli/ (replaces cli.rs, refactors main.rs)

**Files:**
- Create: `src/cli/mod.rs`
- Create: `src/cli/parser.rs`
- Create: `src/cli/commands.rs`
- Modify: `src/main.rs`
- Delete: `src/cli.rs`

- [ ] **Step 1: Create src/cli/parser.rs**

```rust
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
```

- [ ] **Step 2: Create src/cli/commands.rs**

```rust
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
use crate::chain::blockchain::Blockchain;
use crate::crypto::wallet::Wallet;
use crate::crypto::transaction::Transaction;
use crate::types::PublicKey;
use crate::config::Config;
use crate::error::{CliError, CliResult};

pub fn new_wallet(config: &Config) -> CliResult<()> {
    if config.wallet_path.exists() {
        eprintln!("A wallet already exists. Remove {:?} before generating a new one.", config.wallet_path);
        std::process::exit(1);
    }
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let signing_key = SigningKey::from_bytes(&secret);
    let pubkey = PublicKey(signing_key.verifying_key().to_bytes());
    let wallet = Wallet::new(secret, pubkey);
    wallet.save_encrypted(config.wallet_path.to_str().unwrap(), &config.wallet_password)?;
    println!("Generating new wallet...");
    println!("Public key: {}", pubkey);
    Ok(())
}

pub fn show_chain(config: &Config) -> CliResult<()> {
    let blockchain = Blockchain::load(config.chain_path.to_str().unwrap())
        .unwrap_or_default();
    println!("Showing blockchain...");
    for block in blockchain.chain() {
        println!(
            "Block {}: Hash: {}, Prev Hash: {}, Timestamp: {}, Transactions: {}",
            block.index(), block.hash(), block.prev_hash(),
            block.timestamp(), block.transactions().len()
        );
    }
    Ok(())
}

pub fn validate_chain(config: &Config) -> CliResult<()> {
    let blockchain = Blockchain::load(config.chain_path.to_str().unwrap())
        .unwrap_or_default();
    println!("Validating blockchain...");
    println!("Chain is valid: {}", blockchain.validate());
    Ok(())
}

pub fn mine(config: &Config) -> CliResult<()> {
    let chain_path = config.chain_path.to_str().unwrap();
    let mut blockchain = Blockchain::load(chain_path).unwrap_or_default();
    if let Ok(wallet) = Wallet::load_encrypted(
        config.wallet_path.to_str().unwrap(),
        &config.wallet_password,
    ) {
        blockchain.add_coinbase(wallet.pubkey, config.coinbase_reward);
    }
    println!("Mining block...");
    blockchain.mine();
    blockchain.save(chain_path)?;
    println!("Block mined successfully.");
    Ok(())
}

pub fn send(from: &str, to: &str, amount: u64, config: &Config) -> CliResult<()> {
    let chain_path = config.chain_path.to_str().unwrap();
    let mut blockchain = Blockchain::load(chain_path).unwrap_or_default();
    let from_key = PublicKey::try_from(hex::decode(from)?)?;
    let to_key   = PublicKey::try_from(hex::decode(to)?)?;
    let mut tx = Transaction::new(from_key, to_key, amount);
    let wallet = Wallet::load_encrypted(
        config.wallet_path.to_str().unwrap(),
        &config.wallet_password,
    )?;
    tx.sign(&SigningKey::from_bytes(&wallet.secret));
    println!("Sending {} from {} to {}...", amount, from_key, to_key);
    blockchain.add_transaction(tx)?;
    blockchain.save(chain_path)?;
    println!("Transaction in mempool. Run 'mine' to confirm it.");
    Ok(())
}
```

- [ ] **Step 3: Create src/cli/mod.rs**

```rust
pub mod parser;
pub mod commands;

use parser::Command;
use crate::config::Config;
use crate::error::CliResult;

pub fn run(command: Command, config: &Config) -> CliResult<()> {
    match command {
        Command::NewWallet          => commands::new_wallet(config),
        Command::ShowChain          => commands::show_chain(config),
        Command::Validate           => commands::validate_chain(config),
        Command::Mine               => commands::mine(config),
        Command::Send { from, to, amount } => commands::send(&from, &to, amount, config),
    }
}
```

- [ ] **Step 4: Update src/main.rs**

```rust
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
```

- [ ] **Step 5: Add cli module to src/lib.rs**

Add `pub mod cli;` to lib.rs. Remove `pub mod cli;` if it was pointing to old `cli.rs` — the new one is the directory.

- [ ] **Step 6: Delete old src/cli.rs**

```bash
rm src/cli.rs
```

- [ ] **Step 7: Run all tests**

```bash
cargo test 2>&1 && cargo build --bin mini_blockchain 2>&1 | head -20
```
Expected: all tests pass, binary builds.

- [ ] **Step 8: Commit**

```bash
git add src/cli/ src/main.rs src/lib.rs
git rm src/cli.rs
git commit -m "refactor: restructure CLI into cli/ module with thin main.rs"
```

---

## Task 7: Create src/api/ (refactors bin/api.rs)

**Files:**
- Create: `src/api/mod.rs`
- Create: `src/api/handlers/mod.rs`
- Create: `src/api/handlers/chain.rs`
- Create: `src/api/handlers/mining.rs`
- Create: `src/api/handlers/transaction.rs`
- Create: `src/api/handlers/wallet.rs`
- Modify: `src/lib.rs`
- Modify: `src/bin/api.rs`

- [ ] **Step 1: Create src/api/handlers/chain.rs**

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{extract::{State, Path}, Json, http::StatusCode};
use serde_json::Value;
use crate::chain::blockchain::Blockchain;

pub type AppState = Arc<RwLock<Blockchain>>;

pub async fn get_chain(
    State(state): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let bc = state.read().await;
    serde_json::to_value(&*bc)
        .map(Json)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Serialisation error".to_string()))
}

pub async fn validate(
    State(state): State<AppState>,
) -> Result<String, (StatusCode, String)> {
    let bc = state.read().await;
    let result = bc.validate();
    tracing::info!(valid = result, "Chain validated");
    Ok(format!("Chain is valid: {}", result))
}

pub async fn get_block(
    State(state): State<AppState>,
    Path(index): Path<u32>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let bc = state.read().await;
    bc.chain().iter().find(|b| b.index() == index)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Block {} not found", index)))
        .and_then(|block| {
            serde_json::to_value(block)
                .map(Json)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Serialisation error".to_string()))
        })
}
```

- [ ] **Step 2: Create src/api/handlers/wallet.rs**

```rust
use axum::http::StatusCode;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
use crate::crypto::wallet::Wallet;
use crate::types::PublicKey;

pub async fn new_wallet(
    wallet_path: &str,
    password: &str,
) -> Result<String, (StatusCode, String)> {
    if std::path::Path::new(wallet_path).exists() {
        return Err((StatusCode::CONFLICT,
            format!("A wallet already exists at {}. Remove it before generating a new one.", wallet_path)));
    }
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let signing_key = SigningKey::from_bytes(&secret);
    let pubkey = PublicKey(signing_key.verifying_key().to_bytes());
    Wallet::new(secret, pubkey)
        .save_encrypted(wallet_path, password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    tracing::info!(%pubkey, "New wallet generated");
    Ok(pubkey.to_string())
}
```

- [ ] **Step 3: Create src/api/handlers/transaction.rs**

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{extract::State, Json, http::StatusCode};
use serde::Deserialize;
use ed25519_dalek::SigningKey;
use crate::chain::blockchain::Blockchain;
use crate::crypto::{transaction::Transaction, wallet::Wallet};
use crate::types::PublicKey;
use super::chain::AppState;

#[derive(Deserialize)]
pub struct SendPayload {
    pub from: String,
    pub to: String,
    pub amount: u64,
}

pub async fn add_to_mempool(
    State(state): State<AppState>,
    wallet_path: &str,
    password: &str,
    Json(payload): Json<SendPayload>,
) -> Result<String, (StatusCode, String)> {
    let from = PublicKey::try_from(
        hex::decode(&payload.from).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid hex in 'from'".to_string()))?
    ).map_err(|_| (StatusCode::BAD_REQUEST, "'from' must be 32 bytes".to_string()))?;

    let to = PublicKey::try_from(
        hex::decode(&payload.to).map_err(|_| (StatusCode::BAD_REQUEST, "Invalid hex in 'to'".to_string()))?
    ).map_err(|_| (StatusCode::BAD_REQUEST, "'to' must be 32 bytes".to_string()))?;

    let wallet = Wallet::load_encrypted(wallet_path, password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    let mut tx = Transaction::new(from, to, payload.amount);
    tx.sign(&SigningKey::from_bytes(&wallet.secret));

    state.write().await
        .add_transaction(tx)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    tracing::info!(from = %from, to = %to, amount = payload.amount, "Transaction added to mempool");
    Ok("Transaction submitted".to_string())
}
```

- [ ] **Step 4: Create src/api/handlers/mining.rs**

```rust
use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{extract::State, http::StatusCode};
use crate::chain::{blockchain::Blockchain, block::Block};
use crate::crypto::wallet::Wallet;
use super::chain::AppState;

pub async fn mine(
    State(state): State<AppState>,
    chain_path: &str,
    wallet_path: &str,
    password: &str,
    coinbase_reward: u64,
) -> Result<String, (StatusCode, String)> {
    let (index, prev_hash, txs, difficulty) = {
        let mut bc = state.write().await;
        if let Ok(wallet) = Wallet::load_encrypted(wallet_path, password) {
            bc.add_coinbase(wallet.pubkey, coinbase_reward);
        }
        let (tip_index, tip_hash) = bc.tip()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Empty chain".to_string()))?;
        let txs = bc.take_mempool();
        let difficulty = bc.difficulty();
        (tip_index + 1, tip_hash, txs, difficulty)
    };

    let block = tokio::task::spawn_blocking(move || {
        let mut b = Block::new(index, txs, &prev_hash);
        b.mine(difficulty);
        b
    }).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Mining thread error".to_string()))?;

    let mut bc = state.write().await;
    bc.push_block(block);
    bc.save(chain_path)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!("New block mined");
    Ok("Block mined successfully\n".to_string())
}
```

- [ ] **Step 5: Create src/api/handlers/mod.rs**

```rust
pub mod chain;
pub mod mining;
pub mod transaction;
pub mod wallet;
```

- [ ] **Step 6: Create src/api/mod.rs**

All handlers receive `State<AppState>`. `AppState` holds both the blockchain and the config so no closures need to capture loose strings.

```rust
pub mod handlers;

use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{Router, routing::{get, post}};
use tracing_subscriber::EnvFilter;
use crate::chain::blockchain::Blockchain;
use crate::config::{Config, API_BIND_ADDR};

/// Shared state injected into every axum handler via `State<AppState>`.
pub struct ApiState {
    pub blockchain: RwLock<Blockchain>,
    pub config:     Config,
}

pub type AppState = Arc<ApiState>;

pub async fn serve(config: Config) {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let blockchain = Blockchain::load(config.chain_path.to_str().unwrap())
        .unwrap_or_default();

    let state: AppState = Arc::new(ApiState {
        blockchain: RwLock::new(blockchain),
        config,
    });

    let app = Router::new()
        .route("/chain",        get(handlers::chain::get_chain))
        .route("/validate",     get(handlers::chain::validate))
        .route("/block/:index", get(handlers::chain::get_block))
        .route("/wallet",       post(handlers::wallet::new_wallet))
        .route("/transaction",  post(handlers::transaction::add_to_mempool))
        .route("/mine",         post(handlers::mining::mine))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(API_BIND_ADDR).await.unwrap();
    tracing::info!(addr = "http://localhost:3000", "API started");
    axum::serve(listener, app).await.unwrap();
}
```

- [ ] **Step 6b: Update handler signatures to use AppState**

Update `src/api/handlers/chain.rs` — replace `AppState` type alias and imports:
```rust
use axum::{extract::{State, Path}, Json, http::StatusCode};
use serde_json::Value;
use crate::api::AppState;

pub async fn get_chain(State(s): State<AppState>) -> Result<Json<Value>, (StatusCode, String)> {
    let bc = s.blockchain.read().await;
    serde_json::to_value(&*bc)
        .map(Json)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Serialisation error".to_string()))
}

pub async fn validate(State(s): State<AppState>) -> String {
    let bc = s.blockchain.read().await;
    let result = bc.validate();
    tracing::info!(valid = result, "Chain validated");
    format!("Chain is valid: {}", result)
}

pub async fn get_block(
    State(s): State<AppState>,
    Path(index): Path<u32>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let bc = s.blockchain.read().await;
    bc.chain().iter().find(|b| b.index() == index)
        .ok_or_else(|| (StatusCode::NOT_FOUND, format!("Block {} not found", index)))
        .and_then(|block| {
            serde_json::to_value(block)
                .map(Json)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Serialisation error".to_string()))
        })
}
```

Update `src/api/handlers/wallet.rs`:
```rust
use axum::{extract::State, http::StatusCode};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
use crate::api::AppState;
use crate::crypto::wallet::Wallet;
use crate::types::PublicKey;

pub async fn new_wallet(State(s): State<AppState>) -> Result<String, (StatusCode, String)> {
    let wallet_path = s.config.wallet_path.to_str().unwrap();
    if std::path::Path::new(wallet_path).exists() {
        return Err((StatusCode::CONFLICT,
            format!("Wallet already exists at {}. Remove it first.", wallet_path)));
    }
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let signing_key = SigningKey::from_bytes(&secret);
    let pubkey = PublicKey(signing_key.verifying_key().to_bytes());
    Wallet::new(secret, pubkey)
        .save_encrypted(wallet_path, &s.config.wallet_password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    tracing::info!(%pubkey, "New wallet generated");
    Ok(pubkey.to_string())
}
```

Update `src/api/handlers/transaction.rs`:
```rust
use axum::{extract::State, Json, http::StatusCode};
use serde::Deserialize;
use ed25519_dalek::SigningKey;
use crate::api::AppState;
use crate::crypto::{transaction::Transaction, wallet::Wallet};
use crate::types::PublicKey;

#[derive(Deserialize)]
pub struct SendPayload {
    pub from: String,
    pub to:   String,
    pub amount: u64,
}

pub async fn add_to_mempool(
    State(s): State<AppState>,
    Json(payload): Json<SendPayload>,
) -> Result<String, (StatusCode, String)> {
    let from = parse_pubkey(&payload.from, "from")?;
    let to   = parse_pubkey(&payload.to,   "to")?;

    let wallet = Wallet::load_encrypted(
        s.config.wallet_path.to_str().unwrap(),
        &s.config.wallet_password,
    ).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut tx = Transaction::new(from, to, payload.amount);
    tx.sign(&SigningKey::from_bytes(&wallet.secret));

    s.blockchain.write().await
        .add_transaction(tx)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    tracing::info!(from = %from, to = %to, amount = payload.amount, "Transaction added");
    Ok("Transaction submitted".to_string())
}

fn parse_pubkey(hex_str: &str, field: &str) -> Result<PublicKey, (StatusCode, String)> {
    let bytes = hex::decode(hex_str)
        .map_err(|_| (StatusCode::BAD_REQUEST, format!("Invalid hex in '{field}'")))?;
    PublicKey::try_from(bytes)
        .map_err(|_| (StatusCode::BAD_REQUEST, format!("'{field}' must be 32 bytes")))
}
```

Update `src/api/handlers/mining.rs`:
```rust
use axum::{extract::State, http::StatusCode};
use crate::api::AppState;
use crate::chain::block::Block;
use crate::crypto::wallet::Wallet;

pub async fn mine(State(s): State<AppState>) -> Result<String, (StatusCode, String)> {
    let chain_path  = s.config.chain_path.to_str().unwrap().to_string();
    let wallet_path = s.config.wallet_path.to_str().unwrap();
    let reward      = s.config.coinbase_reward;

    let (index, prev_hash, txs, difficulty) = {
        let mut bc = s.blockchain.write().await;
        if let Ok(wallet) = Wallet::load_encrypted(wallet_path, &s.config.wallet_password) {
            bc.add_coinbase(wallet.pubkey, reward);
        }
        let (tip_index, tip_hash) = bc.tip()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Empty chain".to_string()))?;
        let txs = bc.take_mempool();
        let diff = bc.difficulty();
        (tip_index + 1, tip_hash, txs, diff)
    };

    let block = tokio::task::spawn_blocking(move || {
        let mut b = Block::new(index, txs, &prev_hash);
        b.mine(difficulty);
        b
    }).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Mining thread error".to_string()))?;

    let mut bc = s.blockchain.write().await;
    bc.push_block(block);
    bc.save(&chain_path).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!("New block mined");
    Ok("Block mined successfully\n".to_string())
}
```

- [ ] **Step 7: Add api module to src/lib.rs**

Add `pub mod api;` to lib.rs.

- [ ] **Step 8: Update src/bin/api.rs**

```rust
use mini_blockchain::{api, config::Config};

#[tokio::main]
async fn main() {
    api::serve(Config::from_env()).await;
}
```

- [ ] **Step 9: Build the api binary**

```bash
cargo build --bin api 2>&1
```
Expected: builds without errors.

- [ ] **Step 10: Commit**

```bash
git add src/api/ src/lib.rs src/bin/api.rs
git commit -m "refactor: move API handlers into api/ module with thin binary"
```

---

## Task 8: Create src/node/ (refactors bin/node.rs)

**Files:**
- Create: `src/node/mod.rs`
- Create: `src/node/behaviour.rs`
- Create: `src/node/events.rs`
- Create: `src/node/commands.rs`
- Modify: `src/lib.rs`
- Modify: `src/bin/node.rs`

- [ ] **Step 1: Create src/node/behaviour.rs**

```rust
use libp2p::{gossipsub, mdns, request_response, swarm::NetworkBehaviour};
use serde::{Serialize, Deserialize};
use crate::chain::blockchain::Blockchain;

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    pub gossipsub:        gossipsub::Behaviour,
    pub mdns:             mdns::tokio::Behaviour,
    pub request_response: request_response::cbor::Behaviour<ChainRequest, ChainResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainRequest;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub chain: Blockchain,
}
```

- [ ] **Step 2: Create src/node/mod.rs**

```rust
pub mod behaviour;
pub mod events;
pub mod commands;

use std::collections::HashSet;
use std::time::Duration;
use libp2p::{
    gossipsub, mdns, noise, tcp, yamux,
    swarm::SwarmEvent,
    SwarmBuilder,
    request_response::{self, ProtocolSupport},
    StreamProtocol, PeerId,
};
use libp2p::futures::StreamExt;
use tokio::io::{self, AsyncBufReadExt};
use tracing::error;
use crate::chain::blockchain::Blockchain;
use crate::config::Config;
use crate::error::{NodeError, NodeResult};
use behaviour::NodeBehaviour;

/// All mutable state shared between event and command handlers.
pub struct NodeState {
    pub swarm:        libp2p::Swarm<NodeBehaviour>,
    pub blockchain:   Blockchain,
    pub synced_peers: HashSet<PeerId>,
    pub block_topic:  gossipsub::IdentTopic,
    pub tx_topic:     gossipsub::IdentTopic,
    pub chain_path:   String,
}

pub struct Node {
    state: NodeState,
}

impl Node {
    pub fn new(config: Config) -> NodeResult<Self> {
        let swarm = SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)?
            .with_behaviour(|key| {
                let gossipsub_config = gossipsub::ConfigBuilder::default().build()?;
                let gossipsub = gossipsub::Behaviour::new(
                    gossipsub::MessageAuthenticity::Signed(key.clone()),
                    gossipsub_config,
                )?;
                let request_response = request_response::cbor::Behaviour::new(
                    [(StreamProtocol::new("/blockchain/1.0.0"), ProtocolSupport::Full)],
                    request_response::Config::default(),
                );
                let mdns = mdns::tokio::Behaviour::new(
                    mdns::Config::default(),
                    key.public().to_peer_id(),
                )?;
                Ok(NodeBehaviour { gossipsub, mdns, request_response })
            })
            .map_err(|e| NodeError::Transport(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        let chain_path = config.chain_path.to_str().unwrap().to_string();
        let blockchain = Blockchain::load(&chain_path).unwrap_or_default();

        Ok(Self {
            state: NodeState {
                swarm,
                blockchain,
                synced_peers: HashSet::new(),
                block_topic: gossipsub::IdentTopic::new("blocks"),
                tx_topic:    gossipsub::IdentTopic::new("transactions"),
                chain_path,
            },
        })
    }

    pub async fn run(mut self) -> NodeResult<()> {
        self.state.swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap()).unwrap();

        {
            let s = &mut self.state;
            s.swarm.behaviour_mut().gossipsub.subscribe(&s.block_topic).unwrap();
            s.swarm.behaviour_mut().gossipsub.subscribe(&s.tx_topic).unwrap();
        }

        let mut stdin = io::BufReader::new(io::stdin()).lines();

        loop {
            tokio::select! {
                event = self.state.swarm.select_next_some() => {
                    events::handle_swarm_event(event, &mut self.state);
                }
                line = stdin.next_line() => {
                    match line {
                        Ok(Some(line)) => commands::handle_stdin_command(&line, &mut self.state),
                        Ok(None)       => break,
                        Err(e)         => { error!("Stdin read error: {e}"); break; }
                    }
                }
            }
        }
        Ok(())
    }
}
```

- [ ] **Step 3: Create src/node/events.rs**

```rust
use libp2p::{gossipsub, mdns, request_response, swarm::SwarmEvent};
use tracing::{error, info, warn};
use crate::node::behaviour::{NodeBehaviourEvent, ChainResponse};
use crate::node::NodeState;

pub fn handle_swarm_event(
    event: SwarmEvent<NodeBehaviourEvent>,
    state: &mut NodeState,
) {
    match event {
        SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(
            mdns::Event::Discovered(peers)
        )) => {
            for (peer_id, addr) in peers {
                state.swarm.add_peer_address(peer_id, addr);
                state.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                if let Err(e) = state.swarm.dial(peer_id) {
                    warn!("Failed to dial {peer_id}: {e:?}");
                } else {
                    info!("Peer discovered, connecting: {peer_id}");
                }
            }
        }
        SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(
            mdns::Event::Expired(peers)
        )) => {
            for (peer_id, _) in peers {
                state.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                info!("Peer expired: {peer_id}");
            }
        }
        SwarmEvent::ConnectionEstablished { peer_id, .. } => {
            if state.synced_peers.insert(peer_id) {
                state.swarm.behaviour_mut().request_response
                    .send_request(&peer_id, crate::node::behaviour::ChainRequest);
                info!("Connection established with: {peer_id}");
            }
        }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            state.synced_peers.remove(&peer_id);
            info!("Connection closed with: {peer_id}");
        }
        SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(
            gossipsub::Event::Message { propagation_source, message, .. }
        )) => {
            handle_gossip_message(propagation_source, message, state);
        }
        SwarmEvent::Behaviour(NodeBehaviourEvent::RequestResponse(
            request_response::Event::Message { peer, message }
        )) => {
            handle_request_response(peer, message, state);
        }
        SwarmEvent::Behaviour(NodeBehaviourEvent::RequestResponse(
            request_response::Event::OutboundFailure { peer, error, .. }
        )) => {
            error!("Request error to {peer}: {error:?}");
        }
        _ => {}
    }
}

fn handle_gossip_message(
    source: libp2p::PeerId,
    message: gossipsub::Message,
    state: &mut NodeState,
) {
    let block_hash = state.block_topic.hash();
    let tx_hash    = state.tx_topic.hash();

    if message.topic == block_hash {
        if let Ok(block) = serde_json::from_slice::<crate::chain::block::Block>(&message.data) {
            let tip_hash = state.blockchain.chain().last().map(|b| b.hash().clone());
            let valid = block.hash() == &block.compute_hash()
                && tip_hash.as_ref() == Some(block.prev_hash());
            if valid {
                state.blockchain.push_block(block);
                state.blockchain.save(&state.chain_path).ok();
                info!("Block received and appended from {source}");
            } else {
                warn!("Block from {source} rejected (invalid hash or wrong prev_hash)");
            }
        }
    } else if message.topic == tx_hash {
        if let Ok(tx) = serde_json::from_slice::<crate::crypto::transaction::Transaction>(&message.data) {
            match state.blockchain.add_transaction(tx) {
                Ok(_)  => info!("Transaction received from {source}"),
                Err(e) => warn!("Transaction from {source} rejected: {e}"),
            }
        }
    }
}

fn handle_request_response(
    peer: libp2p::PeerId,
    message: request_response::Message<
        crate::node::behaviour::ChainRequest,
        ChainResponse,
    >,
    state: &mut NodeState,
) {
    match message {
        request_response::Message::Request { channel, .. } => {
            let response = ChainResponse { chain: state.blockchain.clone() };
            state.swarm.behaviour_mut().request_response
                .send_response(channel, response).ok();
            info!("Chain sent to {peer}");
        }
        request_response::Message::Response { response, .. } => {
            let received = response.chain;
            if received.chain().len() > state.blockchain.chain().len() && received.validate() {
                info!("Longer chain adopted from {peer}: {} blocks", received.chain().len());
                state.blockchain = received;
                state.blockchain.save(&state.chain_path).ok();
            } else {
                info!("Chain from {peer} discarded (not longer or invalid)");
            }
        }
    }
}
```

- [ ] **Step 4: Create src/node/commands.rs**

```rust
use tracing::{info, warn};
use crate::node::NodeState;
use crate::crypto::transaction::Transaction;
use crate::types::PublicKey;

pub fn handle_stdin_command(line: &str, state: &mut NodeState) {
    if line.starts_with("tx ") {
        handle_tx_command(line, state);
    } else if line.trim() == "mine" {
        handle_mine_command(state);
    }
}

fn handle_tx_command(line: &str, state: &mut NodeState) {
    let parts: Vec<&str> = line.splitn(4, ' ').collect();
    if parts.len() != 4 { return; }

    let amount = match parts[3].trim().parse::<u64>() {
        Ok(a) => a,
        Err(_) => return,
    };
    let from = hex::decode(parts[1]).ok()
        .and_then(|b| PublicKey::try_from(b).ok());
    let to = hex::decode(parts[2]).ok()
        .and_then(|b| PublicKey::try_from(b).ok());

    if let (Some(from), Some(to)) = (from, to) {
        let tx = Transaction::new(from, to, amount);
        if let Ok(bytes) = serde_json::to_vec(&tx) {
            state.swarm.behaviour_mut().gossipsub
                .publish(state.tx_topic.clone(), bytes).ok();
        }
        match state.blockchain.add_transaction(tx) {
            Ok(_)  => info!("Transaction propagated"),
            Err(e) => warn!("Transaction rejected locally: {e}"),
        }
    }
}

fn handle_mine_command(state: &mut NodeState) {
    state.blockchain.mine();
    if let Some(block) = state.blockchain.chain().last() {
        if let Ok(bytes) = serde_json::to_vec(block) {
            state.swarm.behaviour_mut().gossipsub
                .publish(state.block_topic.clone(), bytes).ok();
            info!("Block mined and propagated");
        }
    }
    state.blockchain.save(&state.chain_path).ok();
}
```

- [ ] **Step 5: Add node module to src/lib.rs**

Add `pub mod node;` to lib.rs.

- [ ] **Step 6: Update src/bin/node.rs**

```rust
use mini_blockchain::{node::Node, config::Config, error::NodeError};

#[tokio::main]
async fn main() -> Result<(), NodeError> {
    tracing_subscriber::fmt::init();
    Node::new(Config::from_env())?.run().await
}
```

- [ ] **Step 7: Build the node binary**

```bash
cargo build --bin node 2>&1
```
Expected: builds without errors.

- [ ] **Step 8: Run all tests one final time**

```bash
cargo test 2>&1
```
Expected: all tests pass.

- [ ] **Step 9: Commit**

```bash
git add src/node/ src/lib.rs src/bin/node.rs
git commit -m "refactor: move P2P node into node/ module with thin binary"
```

---

## Final Verification

- [ ] **Run full test suite**

```bash
cargo test 2>&1
```
Expected: all tests pass.

- [ ] **Build all binaries**

```bash
cargo build 2>&1
```
Expected: `mini_blockchain`, `api`, `node` all build without errors or warnings.

- [ ] **Run clippy**

```bash
cargo clippy -- -D warnings 2>&1
```
Fix any lints before completing.

- [ ] **Final commit**

```bash
git add -A
git commit -m "chore: clean code refactor complete"
```
