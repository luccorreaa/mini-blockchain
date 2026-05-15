use axum::{extract::{State, Path}, Json};
use serde_json::Value;
use crate::api::{AppState, error::ApiError};

pub async fn get_chain(State(s): State<AppState>) -> Result<Json<Value>, ApiError> {
    let bc = s.blockchain.read().await;
    serde_json::to_value(&*bc)
        .map(Json)
        .map_err(ApiError::from)
}

pub async fn validate(State(s): State<AppState>) -> String {
    let bc = s.blockchain.read().await;
    let result = bc.validate();
    tracing::info!(valid = result, "Chain validated");
    format!("Chain is valid: {}", result)
}

pub async fn get_block(
    State(s): State<AppState>,
    Path(index): Path<u32>,
) -> Result<Json<Value>, ApiError> {
    let bc = s.blockchain.read().await;
    bc.chain().iter().find(|b| b.index() == index)
        .ok_or_else(|| ApiError::NotFound(format!("Block {} not found", index)))
        .and_then(|block| {
            serde_json::to_value(block)
                .map(Json)
                .map_err(ApiError::from)
        })
}
