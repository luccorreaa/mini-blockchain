//! `mini_blockchain` — a minimal blockchain implementation in Rust.
//!
//! # Crate layout
//!
//! | Module | Responsibility |
//! |--------|----------------|
//! | [`chain`]  | Block, Blockchain, and Merkle tree |
//! | [`crypto`] | Transaction primitives and wallet (Ed25519 + BIP-39 + AES-256-GCM) |
//! | [`cli`]    | CLI commands dispatched via clap |
//! | [`api`]    | REST API (axum): chain, mining, pre-signed transactions |
//! | [`node`]   | P2P node (libp2p): mDNS discovery, Gossipsub, chain sync |
//! | [`config`] | Shared constants and `Config` struct |
//! | [`error`]  | Typed error enums for each layer |

pub mod error;
pub mod types;
pub mod config;
pub mod chain;
pub mod crypto;
pub mod cli;
pub mod api;
pub mod node;
