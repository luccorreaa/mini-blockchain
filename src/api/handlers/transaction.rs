use axum::{extract::State, Json, http::StatusCode};
use serde::Deserialize;
use ed25519_dalek::SigningKey;
use crate::api::AppState;
use crate::crypto::{transaction::Transaction, wallet::Wallet};
use crate::types::PublicKey;

#[derive(Deserialize)]
pub struct SendPayload {
    pub from: String,
    pub to:   String,
    pub amount: u64,
}

pub async fn add_to_mempool(
    State(s): State<AppState>,
    Json(payload): Json<SendPayload>,
) -> Result<String, (StatusCode, String)> {
    let from = parse_pubkey(&payload.from, "from")?;
    let to   = parse_pubkey(&payload.to,   "to")?;

    let wallet = Wallet::load_encrypted(&s.config.wallet_path, &s.config.wallet_password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    let mut tx = Transaction::new(from, to, payload.amount);
    tx.sign(&SigningKey::from_bytes(wallet.secret()));

    s.blockchain.write().await
        .add_transaction(tx)
        .map_err(|e| (StatusCode::BAD_REQUEST, e.to_string()))?;

    tracing::info!(from = %from, to = %to, amount = payload.amount, "Transaction added");
    Ok("Transaction submitted".to_string())
}

fn parse_pubkey(hex_str: &str, field: &str) -> Result<PublicKey, (StatusCode, String)> {
    let bytes = hex::decode(hex_str)
        .map_err(|_| (StatusCode::BAD_REQUEST, format!("Invalid hex in '{field}'")))?;
    PublicKey::try_from(bytes)
        .map_err(|_| (StatusCode::BAD_REQUEST, format!("'{field}' must be 32 bytes")))
}
