# Module Reorganization Design

**Date:** 2026-05-10
**Status:** Approved

## Goal

Reorganize `src/` from a flat file structure into domain-grouped modules to support planned feature growth (logging, mempool, proof of work, balance by address, P2P) and improve portfolio presentation.

## Target Structure

```
src/
  chain/
    mod.rs
    block.rs          # moved from src/block.rs
    blockchain.rs     # moved from src/blockchain.rs
    merkle.rs         # moved from src/merkle.rs
  consensus/
    mod.rs
    proof_of_work.rs  # stub for PoW / mining difficulty
  mempool/
    mod.rs            # stub for pending transaction pool
  network/
    mod.rs
    p2p.rs            # stub for peer-to-peer networking
  wallet/
    mod.rs
    wallet.rs         # moved from src/wallet.rs
    transactions.rs   # moved from src/transactions.rs
    balance.rs        # stub for balance-by-address logic
  bin/
    api.rs            # unchanged
  cli.rs              # unchanged
  main.rs             # unchanged
  lib.rs              # updated to declare the 5 domain modules
```

## Module Responsibilities

- **chain**: Core data structures — Block, Blockchain, Merkle tree. No I/O, no crypto.
- **consensus**: Validation and mining rules. proof_of_work.rs will implement difficulty target and nonce search.
- **mempool**: In-memory pool of unconfirmed transactions waiting to be mined.
- **network**: P2P layer — peer discovery, block/transaction broadcast.
- **wallet**: Keypair management, transaction signing, address balance calculation.

## Changes Required

### File moves
| From | To |
|------|----|
| `src/block.rs` | `src/chain/block.rs` |
| `src/blockchain.rs` | `src/chain/blockchain.rs` |
| `src/merkle.rs` | `src/chain/merkle.rs` |
| `src/wallet.rs` | `src/wallet/wallet.rs` |
| `src/transactions.rs` | `src/wallet/transactions.rs` |

### Import updates
All `use crate::X` paths update to their new location, e.g.:
- `use crate::block::Block` → `use crate::chain::block::Block`
- `use crate::wallet::Wallet` → `use crate::wallet::wallet::Wallet`

Internal references within the same module use `super::` or relative paths.

### lib.rs
Replace flat module declarations with:
```rust
pub mod chain;
pub mod consensus;
pub mod mempool;
pub mod network;
pub mod wallet;
```

### Stubs
`consensus/proof_of_work.rs`, `mempool/mod.rs`, `network/p2p.rs`, and `wallet/balance.rs` are created as empty modules (`// TODO`) so the structure is in place for future implementation.

## Out of Scope
- No logic changes — this is a pure structural reorganization.
- Logging (tracing), PoW, mempool, balance, and P2P are not implemented here; only their module placeholders are created.
