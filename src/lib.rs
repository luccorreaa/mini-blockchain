//! `mini_blockchain` — a minimal blockchain implementation in Rust.
//!
//! # Crate layout
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`block`] | Block structure and Proof-of-Work mining |
//! | [`blockchain`] | Ordered chain of blocks and mempool management |
//! | [`crypto`] | Transaction primitives and wallet (Ed25519 + AES-256-GCM) |
//! | [`merkle`] | SHA-256 Merkle root for transaction sets |

pub mod error;
pub mod types;
pub mod config;
pub mod block;
pub mod blockchain;
pub mod crypto;
pub mod merkle;
