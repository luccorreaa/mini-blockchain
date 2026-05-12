# Mini Blockchain in Rust

A blockchain implementation built from scratch in Rust, focused on understanding the cryptographic primitives used in real networks like Solana and Bitcoin.

## Features

- **Block chain** — linked blocks via SHA-256 hashes, with full chain validation
- **Structured transactions** — each block contains a `Vec<Transaction>` with sender, receiver, amount, and individual signature
- **Ed25519 digital signatures** — blocks and transactions are signed independently; each transaction is signed by its sender's private key
- **Multiple signers** — each participant has their own keypair generated with `OsRng`, a cryptographically secure random number generator
- **Merkle tree** — transactions in each block are hashed using an iterative bottom-up Merkle tree; the Merkle Root is used in both block hashing and block signing
- **JSON persistence** — the chain can be saved to disk and loaded back with full integrity
- **CLI interface** — interact with the blockchain from the terminal using `clap`
- **REST API** — expose the blockchain over HTTP with `axum` and `tokio`; concurrent read access via `Arc<RwLock<Blockchain>>`
- **Proof of Work** — configurable difficulty; blocks are mined by incrementing a nonce until the hash starts with N zero characters
- **Mempool** — transactions are staged in a pending pool before being confirmed in a block via mining
- **Balance tracking** — `balance_of()` scans the chain and pending mempool to compute spendable balance; double-spend attempts are rejected
- **Coinbase transactions** — miners receive a 50-token block reward via a special unsigned transaction (sender = zero key)
- **Anti-replay nonce** — each transaction includes a cryptographically random `u64` nonce included in its signature, preventing replay attacks
- **AES-256-GCM wallet encryption** — private keys are stored on disk encrypted with AES-256-GCM; key is derived from a password via SHA-256
- **Structured logging** — request and event logs via `tracing` + `tracing-subscriber`; log level controlled with `RUST_LOG`

## Cryptographic Primitives

| Primitive | Usage |
|---|---|
| SHA-256 (`sha2`) | Block hash, chain linking, Merkle tree nodes, wallet key derivation |
| Ed25519 (`ed25519-dalek`) | Transaction and block signing/verification |
| AES-256-GCM (`aes-gcm`) | Symmetric encryption of the wallet's private key at rest |
| `OsRng` (`rand`) | Cryptographically secure key generation and nonce generation |

## Project Structure

```
src/
├── lib.rs            # Library crate — shared modules
├── main.rs           # Binary: CLI dispatch
├── bin/
│   └── api.rs        # Binary: REST API (axum + tokio)
├── block.rs          # Block struct with hash calculation, PoW mining, signing, getters
├── blockchain.rs     # Blockchain struct with mempool, balance tracking, validation, persistence
├── transactions.rs   # Transaction struct with individual Ed25519 signatures and nonce
├── merkle.rs         # Merkle tree — iterative bottom-up construction, handles odd counts
├── wallet.rs         # Wallet keypair, AES-256-GCM encryption, saved to disk as JSON
└── cli.rs            # CLI commands: new-wallet, show-chain, validate, send, mine
```

## How It Works

### Block structure

Each block contains:
- `index` — position in the chain
- `transacciones` — list of signed transactions
- `hash_previo` — hash of the previous block
- `hash` — SHA-256 of `index + merkle_root + hash_previo + timestamp + nonce`
- `timestamp` — Unix epoch seconds
- `nonce` — counter incremented during Proof of Work mining
- `firma` — Ed25519 signature of the block by its author
- `autor` — public key (32 bytes) of the block's signer

### Transaction structure

Each transaction contains:
- `sender` — public key of the sender (32 bytes, stored as hex in JSON); all-zero key denotes a coinbase
- `receiver` — public key of the receiver (32 bytes, stored as hex in JSON)
- `amount` — amount transferred (`u64`)
- `nonce` — random `u64` included in the signature to prevent replay attacks
- `firma` — Ed25519 signature of `sender + receiver + amount + nonce` by the sender

### Merkle tree

Transactions within a block are hashed using an iterative bottom-up Merkle tree:

```
           Merkle Root
           [Hash(AB|CD)]
           /           \
     [Hash(AB)]      [Hash(CD)]
       /    \           /    \
  [Hash(A)] [Hash(B)] [Hash(C)] [Hash(D)]
      |         |         |         |
     tx1       tx2       tx3       tx4
```

Each leaf is the SHA-256 of a transaction's `sender + receiver + amount`. Pairs are concatenated and hashed level by level until one hash remains — the Merkle Root. If a level has an odd number of nodes, the last one is duplicated.

The Merkle Root is used in both `calcular_hash` and `firmar`, ensuring that any change to any transaction invalidates both the block hash and the block signature.

### Proof of Work

Mining increments the block's `nonce` field and recalculates the SHA-256 hash until it starts with `difficulty` zero characters:

```
target  = "0".repeat(difficulty)   // e.g. "00" for difficulty 2
loop:
    nonce += 1
    hash   = SHA-256(index + merkle_root + prev_hash + timestamp + nonce)
until hash.starts_with(target)
```

Difficulty is stored in the `Blockchain` struct and defaults to `2`. In the API server, the heavy PoW loop runs inside `tokio::task::spawn_blocking` so it never blocks the async runtime.

### Mempool & Transaction lifecycle

```
send (CLI / POST /transaction)
  └── balance check → add to mempool

mine (CLI / POST /mine)
  ├── add coinbase reward (50 tokens) at position 0
  ├── drain mempool
  ├── run Proof of Work
  └── push block → save to disk
```

### Balance tracking

`Blockchain::balance_of(pubkey)` walks every confirmed block and every pending mempool entry, summing credits and debits. `add_transaction()` calls `balance_of` before accepting a new transaction, rejecting it if the sender's available balance is insufficient.

### Wallet encryption

Wallets are stored encrypted with AES-256-GCM:

1. A 32-byte AES key is derived from the user's password via SHA-256.
2. A fresh 12-byte nonce is generated with `OsRng` on every save.
3. Only the 32-byte private key is encrypted; the public key is stored alongside the ciphertext in plaintext (it is not secret).

The `WALLET_PASSWORD` environment variable provides the password. If unset, the default `dev_password_change_me` is used (suitable for development only).

### Chain validation

`Blockchain::validar()` checks every block:

1. The stored hash matches the recalculated hash (integrity)
2. `hash_previo` matches the actual hash of the previous block (chain linking)
3. Every non-coinbase transaction has a valid Ed25519 signature against its sender key
4. If the block is signed, the block's Ed25519 signature is valid against the stored author key

## CLI Usage

```bash
# Generate a new wallet (saved encrypted to wallet.json)
cargo run --bin mini_blockchain -- new-wallet

# Send a transaction (adds to mempool, checks balance)
cargo run --bin mini_blockchain -- send --from <sender_pubkey_hex> --to <receiver_pubkey_hex> --amount <amount>

# Mine pending transactions into a new block
cargo run --bin mini_blockchain -- mine

# Show all blocks
cargo run --bin mini_blockchain -- show-chain

# Validate the chain
cargo run --bin mini_blockchain -- validate
```

> Set `WALLET_PASSWORD=<your_password>` before running any command that reads or writes `wallet.json`.

## REST API

```bash
# Start the API server
cargo run --bin api

# Control log level
RUST_LOG=debug cargo run --bin api
```

| Method | Endpoint        | Description                                                         |
|--------|-----------------|---------------------------------------------------------------------|
| GET    | `/chain`        | Returns the full blockchain as JSON                                 |
| GET    | `/validar`      | Validates chain integrity                                           |
| GET    | `/block/:index` | Returns a specific block by index                                   |
| POST   | `/wallet`       | Generates a new wallet, returns pubkey (409 if wallet.json exists)  |
| POST   | `/transaction`  | Signs a transaction and adds it to the mempool                      |
| POST   | `/mine`         | Mines pending mempool transactions into a new block                 |

### POST /transaction — Request body

```json
{
  "from": "<sender_pubkey_hex>",
  "to": "<receiver_pubkey_hex>",
  "amount": 100
}
```

### Example flow

```bash
# Generate a wallet
curl -X POST http://localhost:3000/wallet

# Stage a transaction in the mempool
curl -X POST http://localhost:3000/transaction \
  -H "Content-Type: application/json" \
  -d '{"from":"<pubkey>","to":"<pubkey>","amount":100}'

# Mine pending transactions
curl -X POST http://localhost:3000/mine

# Inspect the chain
curl http://localhost:3000/chain

# Get a specific block
curl http://localhost:3000/block/0
```

## Dependencies

```toml
[dependencies]
sha2 = "0.10"
hex = { version = "0.4", features = ["serde"] }
ed25519-dalek = "2.0"
rand = "0.8"
serde = { version = "1.0", features = ["derive"] }
serde_json = "1.0"
clap = { version = "4", features = ["derive"] }
axum = "0.7"
tokio = { version = "1", features = ["full"] }
tracing = "0.1"
tracing-subscriber = { version = "0.3", features = ["env-filter"] }
aes-gcm = "0.10"
```

## Roadmap

- [x] Block chain with SHA-256 linking
- [x] Ed25519 block signatures
- [x] Structured transactions with individual signatures
- [x] Multiple signers
- [x] JSON persistence with serde
- [x] Merkle tree for transaction hashing
- [x] CLI interface with clap
- [x] REST API with axum
- [x] Proof of Work with configurable difficulty
- [x] Mempool for pending transactions
- [x] Balance tracking and double-spend prevention
- [x] Coinbase transactions (block rewards)
- [x] Anti-replay nonce on transactions
- [x] AES-256-GCM wallet encryption
- [x] Structured logging with tracing
- [x] Mempool for pending transactions
- [x] Proof of Work mining
- [] Mempool for pending transactions
- [] Proof of Work mining
