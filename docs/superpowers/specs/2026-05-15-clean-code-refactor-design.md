# Clean Code Refactor — Design Spec

**Date:** 2026-05-15  
**Status:** Approved  
**Scope:** Full refactor of mini_blockchain for clean code, idiomatic Rust patterns, and proper project structure.

---

## 1. Folder Structure

All business logic moves into the lib crate. Binaries become thin entry points (~10 lines each).

```
src/
├── error.rs              ← per-module error enums (thiserror)
├── types.rs              ← Hash, PublicKey newtypes
├── config.rs             ← Config struct + constants
│
├── chain/
│   ├── mod.rs
│   ├── block.rs
│   ├── blockchain.rs
│   └── merkle.rs
│
├── crypto/
│   ├── mod.rs
│   ├── transaction.rs
│   └── wallet.rs
│
├── cli/
│   ├── mod.rs            ← run(command, config) dispatcher
│   ├── parser.rs         ← Cli, Command structs (clap) with global --wallet/--chain flags
│   └── commands.rs       ← one fn per subcommand
│
├── api/
│   ├── mod.rs            ← build_router(), serve(), AppState
│   └── handlers/
│       ├── mod.rs
│       ├── chain.rs      ← get_chain, get_block, validate
│       ├── mining.rs     ← mine (3-phase lock-free PoW)
│       ├── transaction.rs← add_to_mempool
│       └── wallet.rs     ← new_wallet
│
├── node/
│   ├── mod.rs            ← Node struct with new() + run()
│   ├── behaviour.rs      ← NodeBehaviour, ChainRequest, ChainResponse
│   ├── events.rs         ← handle_swarm_event(event, state)
│   └── commands.rs       ← handle_stdin_command(line, state)
│
├── lib.rs                ← re-exports all public modules
│
└── bin/
    ├── api.rs            ← setup tracing + mini_blockchain::api::serve()
    ├── node.rs           ← mini_blockchain::node::Node::new()?.run().await
    └── main.rs           ← parse CLI + mini_blockchain::cli::run()
```

**Principle:** binaries contain only setup and startup. No logic lives in `fn main()`.

---

## 2. Error Handling (`src/error.rs`)

Three independent error enums, composable via `#[from]`. Uses `thiserror` throughout.
Replaces all `Box<dyn std::error::Error>`, `String` errors, and panicking `.expect()` calls.

```rust
// WalletError — wallet module only, no blockchain knowledge
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

// TransactionError — crypto/signing errors
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

// ChainError — chain-level errors, composes the above
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

// CliError — user-facing input errors, used only in cli/ and bin/main.rs
#[derive(Debug, Error)]
pub enum CliError {
    #[error("invalid hex key: {0}")]
    InvalidHex(#[from] hex::FromHexError),
    #[error("key must be 32 bytes")]
    InvalidKeyLength,
    #[error(transparent)]
    Chain(#[from] ChainError),
    #[error(transparent)]
    Wallet(#[from] WalletError),
}

// NodeError — P2P node startup errors
#[derive(Debug, Error)]
pub enum NodeError {
    #[error("transport error: {0}")]
    Transport(String),
    #[error(transparent)]
    Chain(#[from] ChainError),
}

// Convenience type aliases per module
pub type WalletResult<T> = Result<T, WalletError>;
pub type ChainResult<T>  = Result<T, ChainError>;
pub type CliResult<T>    = Result<T, CliError>;
pub type NodeResult<T>   = Result<T, NodeError>;
```

**Error hierarchy:**
```
CliError
  └── ChainError  (#[from])
        └── TransactionError  (#[from])
WalletError        (independent)
NodeError          (independent, wraps ChainError)
```

---

## 3. Semantic Types (`src/types.rs`)

Newtypes prevent mixing up `String` hashes with other strings, and `[u8; 32]` public keys with raw bytes.

### `Hash`
- Wraps `String` (hex-encoded SHA-256 digest)
- `impl Display`, `impl From<String>`, `impl PartialEq`
- `Hash::empty()` for genesis/initial state
- `Hash::as_str() -> &str`

### `PublicKey`
- Wraps `[u8; 32]` (Ed25519 public key bytes)
- `PublicKey::coinbase()` → `[0u8; 32]` — replaces all `[0u8; 32]` literals used as coinbase sentinel
- `PublicKey::is_coinbase() -> bool` — replaces all `tx.sender == [0u8; 32]` checks
- `impl Display` (hex), `impl TryFrom<Vec<u8>>`, `#[serde(with = "hex")]`

**Impact on existing types:**
| Field | Before | After |
|---|---|---|
| `Block.hash` | `String` | `Hash` |
| `Block.prev_hash` | `String` | `Hash` |
| `Block.author` | `Option<[u8; 32]>` | `Option<PublicKey>` |
| `Transaction.sender` | `pub [u8; 32]` | `PublicKey` (private + getter) |
| `Transaction.receiver` | `pub [u8; 32]` | `PublicKey` (private + getter) |
| `Transaction.signature` | `pub Option<Vec<u8>>` | `Option<Vec<u8>>` (private + getter) |
| `Transaction.amount` | `pub u64` | `u64` (private + getter) |
| `Transaction.nonce` | `pub u64` | `u64` (private + getter) |

---

## 4. Abstractions and Helpers

### `chain/merkle.rs`
Extract `fn sha256_hex(content: &str) -> Hash` as a private helper.
The two inline hasher constructions (leaf and internal nodes) collapse to single calls.

### `chain/blockchain.rs`
Split `validate()` into private helpers:
- `fn validate_block(&self, i: usize, block: &Block) -> bool`
- `fn verify_tx_signature(tx: &Transaction) -> Result<(), TransactionError>`
- `fn verify_block_signature(block: &Block) -> Result<(), ChainError>`

`validate()` becomes a clean `.all()` iterator call over `validate_block`.

`impl Default for Blockchain` — delegates to `Blockchain::new()`.

### `crypto/transaction.rs`
All fields made private. Getters added:
- `sender() -> PublicKey`
- `receiver() -> PublicKey`
- `amount() -> u64`
- `nonce() -> u64`
- `signature() -> Option<&[u8]>`

### `node/events.rs` and `node/commands.rs`
The 130-line `match` in `node.rs` main loop splits into:
- `handle_swarm_event(event, state: &mut NodeState)` 
- `handle_stdin_command(line: &str, state: &mut NodeState)`

`NodeState` holds the swarm, blockchain, synced_peers, and topic handles.

---

## 5. Config (`src/config.rs`)

Eliminates all hardcoded paths and magic numbers.

```rust
pub const DEFAULT_WALLET_PATH: &str = "wallet.json";
pub const DEFAULT_CHAIN_PATH:  &str = "blockchain.json";
pub const DEFAULT_DIFFICULTY:  usize = 2;
pub const COINBASE_REWARD:     u64   = 50;
pub const API_BIND_ADDR:       &str  = "0.0.0.0:3000";

pub struct Config {
    pub wallet_path:    PathBuf,
    pub chain_path:     PathBuf,
    pub difficulty:     usize,
    pub coinbase_reward: u64,
    pub wallet_password: String,
}

impl Config {
    /// Builds config from env vars, falling back to defaults.
    /// WALLET_PASSWORD env var is read here — not scattered across binaries.
    pub fn from_env() -> Self { ... }
}

impl Default for Config {
    fn default() -> Self { Self::from_env() }
}
```

**CLI flags override config for paths:**
```
mini_blockchain --wallet other.json --chain other_chain.json mine
```

---

## 6. CLI Restructure

### `src/cli/parser.rs`
`Cli` gains global `--wallet` and `--chain` flags (with defaults from constants).
`Command` enum unchanged in variants, but lives in its own file.

### `src/cli/commands.rs`
One `pub fn` per command, each receives `&Config`, returns `CliResult<()>`:
- `new_wallet(config)`
- `show_chain(config)`
- `validate_chain(config)`
- `mine(config)`
- `send(from, to, amount, config)`

### `src/cli/mod.rs`
```rust
pub fn run(command: Command, config: &Config) -> CliResult<()> {
    match command {
        Command::NewWallet        => commands::new_wallet(config),
        Command::ShowChain        => commands::show_chain(config),
        Command::Validate         => commands::validate_chain(config),
        Command::Mine             => commands::mine(config),
        Command::Send { from, to, amount } => commands::send(&from, &to, amount, config),
    }
}
```

### `src/bin/main.rs`
```rust
fn main() {
    let cli = Cli::parse();
    let config = Config::from_cli(&cli);
    if let Err(e) = mini_blockchain::cli::run(cli.command, &config) {
        eprintln!("error: {e}");
        std::process::exit(1);
    }
}
```

---

## 7. API Restructure

### `src/api/mod.rs`
```rust
pub type AppState = Arc<RwLock<Blockchain>>;

pub fn build_router(state: AppState) -> Router { ... }

pub async fn serve(config: Config) {
    // setup tracing, load blockchain, build router, bind, serve
}
```

### `src/api/handlers/`
Each file handles one resource group:
- `chain.rs` → `get_chain`, `get_block`, `validate`
- `mining.rs` → `mine` (3-phase: extract → spawn_blocking PoW → push+save)
- `transaction.rs` → `add_to_mempool`
- `wallet.rs` → `new_wallet`

All handlers return `Result<T, (StatusCode, String)>` (unchanged, idiomatic for axum).

### `src/bin/api.rs`
```rust
#[tokio::main]
async fn main() {
    mini_blockchain::api::serve(Config::from_env()).await;
}
```

---

## 8. P2P Node Restructure

### `src/node/behaviour.rs`
```rust
#[derive(NetworkBehaviour)]
pub struct NodeBehaviour { ... }

pub struct ChainRequest;
pub struct ChainResponse { pub chain: Blockchain }
```

### `src/node/mod.rs`
```rust
pub struct NodeState {
    swarm: Swarm<NodeBehaviour>,
    blockchain: Blockchain,
    synced_peers: HashSet<PeerId>,
    block_topic: IdentTopic,
    tx_topic: IdentTopic,
    config: Config,
}

pub struct Node { state: NodeState }

impl Node {
    pub fn new(config: Config) -> NodeResult<Self> { ... }
    pub async fn run(mut self) -> NodeResult<()> {
        loop {
            tokio::select! {
                event = self.state.swarm.select_next_some() =>
                    events::handle_swarm_event(event, &mut self.state),
                line = stdin.next_line() =>
                    commands::handle_stdin_command(&line, &mut self.state),
            }
        }
    }
}
```

### `src/node/events.rs`
`pub fn handle_swarm_event(event, state: &mut NodeState)` — contains the full match, broken into named sub-handlers per event variant.

### `src/node/commands.rs`
`pub fn handle_stdin_command(line: &str, state: &mut NodeState)` — parses `tx` and `mine` commands.

### `src/bin/node.rs`
```rust
#[tokio::main]
async fn main() -> Result<(), NodeError> {
    tracing_subscriber::fmt::init();
    Node::new(Config::from_env())?.run().await
}
```

---

## Out of Scope

- Adding new features or changing blockchain consensus logic
- Changing the JSON serialization format (would break existing `blockchain.json` files)
- Renaming public API methods (already done in commit `d15dd62`)
- Test function names (intentionally kept in Spanish per prior decision)
