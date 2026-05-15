use axum::{extract::State, Json};
use serde::Deserialize;
use crate::api::{AppState, error::ApiError};
use crate::crypto::transaction::Transaction;
use crate::types::PublicKey;

#[derive(Deserialize)]
pub struct SendPayload {
    pub from:      String,
    pub to:        String,
    pub amount:    u64,
    pub nonce:     u64,
    pub signature: String,
}

pub async fn add_to_mempool(
    State(s): State<AppState>,
    Json(payload): Json<SendPayload>,
) -> Result<String, ApiError> {
    let from = parse_pubkey(&payload.from, "from")?;
    let to   = parse_pubkey(&payload.to,   "to")?;
    let sig  = hex::decode(&payload.signature)
        .map_err(|_| ApiError::BadRequest("invalid hex in 'signature'".to_string()))?;

    let tx = Transaction::from_parts(from, to, payload.amount, payload.nonce, sig);
    tx.verify_signature().map_err(ApiError::from)?;

    s.blockchain.write().await
        .add_transaction(tx)
        .map_err(ApiError::from)?;

    tracing::info!(from = %from, to = %to, amount = payload.amount, "Transaction added");
    Ok("Transaction submitted".to_string())
}

fn parse_pubkey(hex_str: &str, field: &str) -> Result<PublicKey, ApiError> {
    let bytes = hex::decode(hex_str)
        .map_err(|_| ApiError::BadRequest(format!("invalid hex in '{field}'")))?;
    PublicKey::try_from(bytes)
        .map_err(|_| ApiError::BadRequest(format!("'{field}' must be 32 bytes")))
}
