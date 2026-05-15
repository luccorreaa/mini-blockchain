//! `mini_blockchain` — a minimal blockchain implementation in Rust.
//!
//! # Crate layout
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`block`] | Block structure and Proof-of-Work mining |
//! | [`blockchain`] | Ordered chain of blocks and mempool management |
//! | [`transactions`] | Transaction primitives and Ed25519 signing |
//! | [`merkle`] | SHA-256 Merkle root for transaction sets |
//! | [`wallet`] | Ed25519 key pair with AES-256-GCM encrypted persistence |

pub mod error;
pub mod block;
pub mod blockchain;
pub mod transactions;
pub mod merkle;
pub mod wallet;
