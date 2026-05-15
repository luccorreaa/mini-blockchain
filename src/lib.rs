//! `mini_blockchain` — a minimal blockchain implementation in Rust.
//!
//! # Crate layout
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`chain`] | Block, Blockchain, and Merkle tree |
//! | [`crypto`] | Transaction primitives and wallet (Ed25519 + AES-256-GCM) |

pub mod error;
pub mod types;
pub mod config;
pub mod chain;
pub mod crypto;
pub mod cli;
