//! HTTP REST API for the mini-blockchain node.
//!
//! Exposes the following endpoints:
//!
//! | Method | Path | Description |
//! |--------|------|-------------|
//! | GET | `/chain` | Return the full chain as JSON |
//! | GET | `/validar` | Validate the chain and return the result |
//! | GET | `/block/:index` | Return a single block by index |
//! | POST | `/wallet` | Generate a new wallet |
//! | POST | `/transaction` | Add a signed transaction to the mempool |
//! | POST | `/mine` | Mine the next block from the mempool |
//!
//! The blockchain is held in an `Arc<RwLock<Blockchain>>` shared across handlers.
//! Mining releases the write lock before running Proof-of-Work so other requests
//! are not blocked during the CPU-intensive hashing loop.

use std::sync::Arc;
use tokio::sync::RwLock;
use mini_blockchain::blockchain::Blockchain;
use mini_blockchain::wallet::Wallet;
use mini_blockchain::transactions::Transaction;
use mini_blockchain::block::Block;
use axum::{Router, routing::{get, post}, extract::State, Json};
use axum::extract::Path;
use axum::http::StatusCode;
use serde_json::Value;
use serde::Deserialize;
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
use tracing::info;
use tracing_subscriber::EnvFilter;

type AppState = Arc<RwLock<Blockchain>>;

/// Request body for the `POST /transaction` endpoint.
#[derive(Deserialize)]
struct SendPayload {
    /// Sender's public key, hex-encoded.
    from: String,
    /// Recipient's public key, hex-encoded.
    to: String,
    /// Amount of units to transfer.
    amount: u64,
}

/// Returns the wallet encryption password from the `WALLET_PASSWORD` environment variable,
/// falling back to an insecure development default when the variable is not set.
fn wallet_password() -> String {
    std::env::var("WALLET_PASSWORD").unwrap_or_else(|_| "dev_password_change_me".to_string())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let blockchain = Blockchain::load("blockchain.json")
        .unwrap_or_else(|_| Blockchain::new());
    let state: AppState = Arc::new(RwLock::new(blockchain));

    let app = Router::new()
        .route("/chain", get(get_chain))
        .route("/validar", get(validate))
        .route("/block/:index", get(get_block))
        .route("/wallet", post(new_wallet))
        .route("/transaction", post(add_to_mempool))
        .route("/mine", post(mine))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!(addr = "http://localhost:3000", "API started");
    axum::serve(listener, app).await.unwrap();
}

/// `GET /chain` — returns the full blockchain serialised as JSON.
async fn get_chain(
    State(blockchain): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let bc = blockchain.read().await;
    Ok(Json(
        serde_json::to_value(&*bc)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Serialisation error".to_string()))?,
    ))
}

/// `POST /wallet` — generates a new Ed25519 wallet and saves it encrypted to `wallet.json`.
///
/// Returns HTTP 409 if `wallet.json` already exists.
async fn new_wallet() -> Result<String, (StatusCode, String)> {
    if std::path::Path::new("wallet.json").exists() {
        return Err((
            StatusCode::CONFLICT,
            "A wallet already exists. Remove wallet.json before generating a new one.".to_string(),
        ));
    }
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let signing_key = SigningKey::from_bytes(&secret);
    let pubkey = signing_key.verifying_key().to_bytes();
    let pubkey_hex = hex::encode(pubkey);
    let w = Wallet::new(secret, pubkey);
    w.save_encrypted("wallet.json", &wallet_password())
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save wallet".to_string()))?;
    info!(pubkey = %pubkey_hex, "New wallet generated");
    Ok(pubkey_hex)
}

/// `POST /transaction` — signs and adds a transaction to the mempool.
///
/// Returns HTTP 400 if the `from` or `to` keys are invalid hex, or if the sender
/// has insufficient balance.
async fn add_to_mempool(
    State(blockchain): State<AppState>,
    Json(payload): Json<SendPayload>,
) -> Result<String, (StatusCode, String)> {
    let from: [u8; 32] = hex::decode(&payload.from)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid 'from' key".to_string()))?
        .try_into()
        .map_err(|_| (StatusCode::BAD_REQUEST, "'from' must be 32 bytes".to_string()))?;

    let to: [u8; 32] = hex::decode(&payload.to)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Invalid 'to' key".to_string()))?
        .try_into()
        .map_err(|_| (StatusCode::BAD_REQUEST, "'to' must be 32 bytes".to_string()))?;

    let wallet = Wallet::load_encrypted("wallet.json", &wallet_password())
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to load wallet".to_string()))?;
    let signing_key = SigningKey::from_bytes(&wallet.secret);

    let mut tx = Transaction::new(from, to, payload.amount);
    tx.sign(&signing_key);

    let mut bc = blockchain.write().await;
    bc.add_transaction(tx).map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    info!(from = %payload.from, to = %payload.to, amount = payload.amount, "Transaction added to mempool");
    Ok("Transaction submitted".to_string())
}

/// `GET /validar` — validates the chain and returns the result as plain text.
async fn validate(
    State(blockchain): State<AppState>,
) -> Result<String, (StatusCode, String)> {
    let bc = blockchain.read().await;
    let result = bc.validate();
    info!(valid = result, "Chain validated");
    Ok(format!("Chain is valid: {}", result))
}

/// `GET /block/:index` — returns a single block by its index.
///
/// Returns HTTP 404 if no block with that index exists.
async fn get_block(
    State(blockchain): State<AppState>,
    Path(index): Path<u32>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let bc = blockchain.read().await;
    match bc.chain().iter().find(|b| b.index() == index) {
        Some(block) => Ok(Json(
            serde_json::to_value(block)
                .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Serialisation error".to_string()))?,
        )),
        None => Err((StatusCode::NOT_FOUND, format!("Block {} not found", index))),
    }
}

/// `POST /mine` — mines the next block from the current mempool.
///
/// Mining runs on a `spawn_blocking` thread to avoid starving the async runtime.
/// The write lock is released before Proof-of-Work begins and re-acquired only to
/// push the finished block and persist the chain.
async fn mine(
    State(blockchain): State<AppState>,
) -> Result<String, (StatusCode, String)> {
    // Phase 1: extract data and release the write lock before the CPU-intensive PoW.
    let (index, prev_hash, txs, difficulty) = {
        let mut bc = blockchain.write().await;

        if let Ok(wallet) = Wallet::load_encrypted("wallet.json", &wallet_password()) {
            bc.add_coinbase(wallet.pubkey, 50);
        }

        let (tip_index, tip_hash) = bc
            .tip()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Empty chain".to_string()))?;
        let txs = bc.take_mempool();
        let difficulty = bc.difficulty();
        (tip_index + 1, tip_hash, txs, difficulty)
    }; // write lock released here

    // Phase 2: mine without holding any lock.
    let block = tokio::task::spawn_blocking(move || {
        let mut b = Block::new(index, txs, &prev_hash);
        b.mine(difficulty);
        b
    })
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Mining thread error".to_string()))?;

    // Phase 3: push the mined block and persist.
    let mut bc = blockchain.write().await;
    bc.push_block(block);
    bc.save("blockchain.json")
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Failed to save blockchain".to_string()))?;

    info!("New block mined");
    Ok("Block mined successfully\n".to_string())
}
