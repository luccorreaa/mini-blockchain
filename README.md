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

## Cryptographic Primitives

| Primitive | Usage |
|---|---|
| SHA-256 (`sha2`) | Block hash, chain linking, Merkle tree nodes |
| Ed25519 (`ed25519-dalek`) | Transaction and block signing/verification |
| `OsRng` (`rand`) | Cryptographically secure key generation |

## Project Structure

```
src/
├── main.rs           # Entry point — CLI dispatch
├── block.rs          # Block struct with hash calculation, signing, getters
├── blockchain.rs     # Blockchain struct with validation and persistence
├── transactions.rs   # Transaction struct with individual Ed25519 signatures
├── merklee.rs        # Merkle tree — iterative bottom-up construction, handles odd counts
├── wallet.rs         # Wallet keypair, saved to disk as JSON
└── cli.rs            # CLI commands: new-wallet, show-chain, validate, send
```

## How It Works

### Block structure

Each block contains:
- `index` — position in the chain
- `transacciones` — list of signed transactions
- `hash_previo` — hash of the previous block
- `hash` — SHA-256 of `index + merkle_root + hash_previo + timestamp`
- `timestamp` — Unix epoch seconds
- `firma` — Ed25519 signature of the block by its author
- `autor` — public key (32 bytes) of the block's signer

### Transaction structure

Each transaction contains:
- `sender` — public key of the sender (32 bytes, stored as hex in JSON)
- `receiver` — public key of the receiver (32 bytes, stored as hex in JSON)
- `amount` — amount transferred (`u64`)
- `firma` — Ed25519 signature of the transaction by the sender

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

### Chain validation

`Blockchain::validar()` checks three things for every block:

1. The stored hash matches the recalculated hash (integrity)
2. `hash_previo` matches the actual hash of the previous block (chain linking)
3. If the block is signed, the Ed25519 signature is valid against the stored public key (authenticity)

## CLI Usage

```bash
# Generate a new wallet (saved to wallet.json)
cargo run -- new-wallet

# Send a transaction
cargo run -- send --from <sender_pubkey_hex> --to <receiver_pubkey_hex> --amount <amount>

# Show all blocks
cargo run -- show-chain

# Validate the chain
cargo run -- validate
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
```

## Roadmap

- [x] Block chain with SHA-256 linking
- [x] Ed25519 block signatures
- [x] Structured transactions with individual signatures
- [x] Multiple signers
- [x] JSON persistence with serde
- [x] Merkle tree for transaction hashing
- [x] CLI interface with clap
- [ ] REST API with axum