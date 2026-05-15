<div align="center">

# ⛓️ mini-blockchain

A blockchain built from scratch in Rust — cryptographic primitives, peer-to-peer networking, and a REST API, all in one project.

<a href="https://www.rust-lang.org/"><img src="https://img.shields.io/badge/rust-2024-orange.svg?logo=rust&logoColor=white" alt="Rust 2024"/></a>
<a href="https://github.com/libp2p/rust-libp2p"><img src="https://img.shields.io/badge/libp2p-0.54-blue.svg" alt="libp2p 0.54"/></a>
<a href="https://docs.rs/axum"><img src="https://img.shields.io/badge/axum-0.7-purple.svg" alt="axum 0.7"/></a>
<a href="LICENSE"><img src="https://img.shields.io/badge/license-MIT-brightgreen.svg" alt="License MIT"/></a>

</div>

---

## What is this

mini-blockchain is a fully functional blockchain node implemented from scratch in Rust. There is no central server. Nodes discover each other on the local network, synchronize their chains automatically, and propagate new blocks and transactions through the mesh.

The project covers three layers:

- **Cryptographic core** — SHA-256 chain linking, Ed25519 transaction and block signatures, Merkle tree, AES-256-GCM wallet encryption, and Proof of Work mining.
- **REST API** — expose the blockchain over HTTP. Concurrent read access via `Arc<RwLock<Blockchain>>`. Mining runs in `spawn_blocking` so it never blocks the async runtime.
- **P2P network** — nodes discover each other via mDNS, synchronize chains using request/response, and broadcast new blocks and transactions with Gossipsub.

---

## Architecture

```
src/
├── lib.rs            # Library crate — shared modules
├── main.rs           # Binary: CLI (clap)
├── bin/
│   ├── api.rs        # Binary: REST API (axum + tokio)
│   └── node.rs       # Binary: P2P node (libp2p)
├── block.rs          # Block struct, PoW mining, signing, hash calculation
├── blockchain.rs     # Chain, mempool, balance tracking, validation, persistence
├── transactions.rs   # Transaction struct, Ed25519 signatures, anti-replay nonce
├── merkle.rs         # Iterative bottom-up Merkle tree
├── wallet.rs         # Keypair generation, AES-256-GCM encryption
└── cli.rs            # CLI commands
```

---

## Cryptographic Primitives

| Primitive | Usage |
|---|---|
| SHA-256 (`sha2`) | Block hash, chain linking, Merkle tree nodes, wallet key derivation |
| Ed25519 (`ed25519-dalek`) | Transaction and block signing/verification |
| AES-256-GCM (`aes-gcm`) | Symmetric encryption of the wallet's private key at rest |
| `OsRng` (`rand`) | Cryptographically secure key generation and nonce generation |

---

## How It Works

### Block structure

Each block contains:
- `index` — position in the chain
- `transactions` — list of signed transactions
- `prev_hash` — hash of the previous block
- `hash` — SHA-256 of `index + merkle_root + prev_hash + timestamp + nonce`
- `timestamp` — Unix epoch seconds
- `nonce` — counter incremented during Proof of Work
- `signature` — Ed25519 signature of the block by its author
- `author` — public key (32 bytes) of the block's signer

### Merkle tree

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

Each leaf is the SHA-256 of a transaction's `sender + receiver + amount`. Pairs are concatenated and hashed level by level until one hash remains. If a level has an odd number of nodes, the last one is duplicated. The Merkle Root is used in both `compute_hash` and `sign`, ensuring any change to any transaction invalidates both the block hash and the block signature.

### Proof of Work

```
target = "0".repeat(difficulty)   // e.g. "00" for difficulty 2
loop:
    nonce += 1
    hash   = SHA-256(index + merkle_root + prev_hash + timestamp + nonce)
until hash.starts_with(target)
```

Difficulty is stored in the `Blockchain` struct and defaults to `2`. In the API server, the PoW loop runs inside `tokio::task::spawn_blocking` so it never blocks the async runtime.

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

### P2P node

```
Node starts
  └── listen on TCP (random port)
  └── subscribe to "blocks" and "transactions" topics

mDNS discovers peer
  └── dial peer
  └── ConnectionEstablished → send ChainRequest

ChainRequest received
  └── respond with full Blockchain

ChainResponse received
  └── if longer and valid → adopt + save

"mine" typed in stdin
  └── mine locally → publish block via Gossipsub

"tx <from> <to> <amount>" typed in stdin
  └── create transaction → publish via Gossipsub

Gossipsub message received
  ├── topic "blocks"       → push_block + save
  └── topic "transactions" → add_transaction
```

### Wallet encryption

1. A 32-byte AES key is derived from the user's password via SHA-256.
2. A fresh 12-byte nonce is generated with `OsRng` on every save.
3. Only the 32-byte private key is encrypted; the public key is stored in plaintext (it is not secret).

The `WALLET_PASSWORD` environment variable provides the password. If unset, `dev_password_change_me` is used (development only).

### Chain validation

`Blockchain::validate()` checks every block:
1. The stored hash matches the recalculated hash
2. `prev_hash` matches the actual hash of the previous block
3. Every non-coinbase transaction has a valid Ed25519 signature against its sender key
4. If the block is signed, the block's Ed25519 signature is valid against the stored author key

---

## CLI Usage

```bash
# Generate a new wallet (saved encrypted to wallet.json)
WALLET_PASSWORD=<password> cargo run --bin mini_blockchain -- new-wallet

# Send a transaction (adds to mempool, checks balance)
WALLET_PASSWORD=<password> cargo run --bin mini_blockchain -- send \
  --from <sender_pubkey_hex> \
  --to <receiver_pubkey_hex> \
  --amount <amount>

# Mine pending transactions into a new block
WALLET_PASSWORD=<password> cargo run --bin mini_blockchain -- mine

# Show all blocks
cargo run --bin mini_blockchain -- show-chain

# Validate the chain
cargo run --bin mini_blockchain -- validate
```

---

## REST API

```bash
# Start the API server
WALLET_PASSWORD=<password> cargo run --bin api

# Control log level
RUST_LOG=debug WALLET_PASSWORD=<password> cargo run --bin api
```

| Method | Endpoint        | Description                                                         |
|--------|-----------------|---------------------------------------------------------------------|
| GET    | `/chain`        | Returns the full blockchain as JSON                                 |
| GET    | `/validar`      | Validates chain integrity                                           |
| GET    | `/block/:index` | Returns a specific block by index                                   |
| POST   | `/wallet`       | Generates a new wallet, returns pubkey (409 if wallet.json exists)  |
| POST   | `/transaction`  | Signs a transaction and adds it to the mempool                      |
| POST   | `/mine`         | Mines pending mempool transactions into a new block                 |

### Example flow

```bash
curl -X POST http://localhost:3000/wallet

curl -X POST http://localhost:3000/transaction \
  -H "Content-Type: application/json" \
  -d '{"from":"<pubkey>","to":"<pubkey>","amount":100}'

curl -X POST http://localhost:3000/mine

curl http://localhost:3000/chain
```

---

## P2P Node

```bash
# Start a node (run multiple instances in separate terminals)
cargo run --bin node
```

Once running, two or more nodes on the same network will discover each other automatically via mDNS, synchronize their chains, and keep each other updated in real time.

Available stdin commands:
```
mine                          mine a block and broadcast it to all peers
tx <from_hex> <to_hex> <amt>  create and broadcast a transaction
```

---

## Dependencies

```toml
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
libp2p = { version = "0.54", features = ["mdns", "gossipsub", "tokio", "tcp", "noise", "yamux", "macros", "request-response", "cbor"] }
```

---

## Roadmap

- [x] Block chain with SHA-256 linking
- [x] Ed25519 block and transaction signatures
- [x] Merkle tree for transaction hashing
- [x] JSON persistence with serde
- [x] CLI interface with clap
- [x] REST API with axum + RwLock + spawn_blocking
- [x] Proof of Work with configurable difficulty
- [x] Mempool and double-spend prevention
- [x] Balance tracking per address
- [x] Coinbase transactions (block rewards)
- [x] Anti-replay nonce on transactions
- [x] AES-256-GCM wallet encryption
- [x] Structured logging with tracing
- [x] P2P networking with libp2p (mDNS + Gossipsub + request/response)
- [ ] Custom error types with thiserror
- [ ] Block and transaction validation on P2P receive