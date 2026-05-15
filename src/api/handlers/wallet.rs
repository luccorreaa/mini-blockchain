use axum::{extract::State, http::StatusCode};
use ed25519_dalek::SigningKey;
use rand::rngs::OsRng;
use rand::RngCore;
use crate::api::AppState;
use crate::crypto::wallet::Wallet;
use crate::types::PublicKey;

pub async fn new_wallet(State(s): State<AppState>) -> Result<String, (StatusCode, String)> {
    if s.config.wallet_path.exists() {
        return Err((StatusCode::CONFLICT,
            format!("Wallet already exists at {}. Remove it first.", s.config.wallet_path.display())));
    }
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let signing_key = SigningKey::from_bytes(&secret);
    let pubkey = PublicKey::from_bytes(signing_key.verifying_key().to_bytes());
    Wallet::new(secret, pubkey)
        .save_encrypted(&s.config.wallet_path, &s.config.wallet_password)
        .map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;
    tracing::info!(%pubkey, "New wallet generated");
    Ok(pubkey.to_string())
}
