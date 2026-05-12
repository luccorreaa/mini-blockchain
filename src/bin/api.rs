// src/bin/api.rs
use std::sync::{Arc, Mutex};
use mini_blockchain::blockchain::{ Blockchain};
use mini_blockchain::wallet::Wallet;
use mini_blockchain::transactions::Transaction;
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

#[derive(Deserialize)]
struct SendPayload {
    from: String,
    to: String,
    amount: u64,
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let blockchain = Blockchain::cargar("blockchain.json")
        .unwrap_or_else(|_| Blockchain::new_blockchain());
    let state = Arc::new(Mutex::new(blockchain));

    let app = Router::new()
        .route("/chain", get(get_chain))
        .route("/validar", get(validar))
        .route("/block/:index", get(get_block))
        .route("/wallet", post(wallet))
        .route("/transaction", post(add_to_mempool))
        .route("/mine", post(mine))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    info!(addr = "http://localhost:3000", "API iniciada");
    axum::serve(listener, app).await.unwrap();
}

async fn get_chain(
    State(blockchain): State<Arc<Mutex<Blockchain>>>
) -> Result<Json<Value>, (StatusCode, String)> {
    let bc = blockchain.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error interno".to_string()))?;
    Ok(Json(serde_json::to_value(&*bc).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al serializar la cadena".to_string()))?))
}

async fn wallet()->String {
        let mut secret = [0u8; 32];
        OsRng.fill_bytes(&mut secret);
        let signing_key = SigningKey::from_bytes(&secret);
        let pubkey = signing_key.verifying_key().to_bytes();
        let pubkey_hex = hex::encode(pubkey);
        let wallet = Wallet::new(secret, pubkey);
        wallet.guardar("wallet.json").expect("Error al guardar la wallet");
        info!(pubkey = %pubkey_hex, "Nueva wallet generada");
        pubkey_hex
}

async fn add_to_mempool(
    State(blockchain): State<Arc<Mutex<Blockchain>>>,
    Json(payload): Json<SendPayload>
) -> Result<String, (StatusCode, String)> {
    let from = hex::decode(&payload.from)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Clave pública 'from' inválida".to_string()))?
        .try_into()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Clave pública 'from' debe ser de 32 bytes".to_string()))?;

    let to = hex::decode(&payload.to)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Clave pública 'to' inválida".to_string()))?
        .try_into()
        .map_err(|_| (StatusCode::BAD_REQUEST, "Clave pública 'to' debe ser de 32 bytes".to_string()))?;

    let mut bc = blockchain.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error interno".to_string()))?;

    let mut tx = Transaction::new(from, to, payload.amount);

    let wallet = Wallet::cargar("wallet.json")
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al cargar la wallet".to_string()))?;

    let signing_key = SigningKey::from_bytes(&wallet.secret);
    tx.firmar(&signing_key);
    bc.add_transaction(tx)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    info!(from = %payload.from, to = %payload.to, amount = payload.amount, "Transacción agregada a mempool");
    Ok("Transacción enviada".to_string())
}

async fn validar(
    State(blockchain): State<Arc<Mutex<Blockchain>>>
) -> Result<String, (StatusCode, String)> {
    let blockchain = blockchain.lock().map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error interno".to_string()))?;
    let resultado = blockchain.validar();
    info!(valida = resultado, "Validación ejecutada");
    Ok(format!("La cadena de bloques es válida: {}", resultado))
}   

async fn get_block(
    State(blockchain): State<Arc<Mutex<Blockchain>>>,
    Path(index): Path<u32>
) -> Result<Json<Value>, (StatusCode, String)> {
    let blockchain = blockchain.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error interno".to_string()))?;
    
    match blockchain.cadena().iter().find(|b| b.index() == index) {
        Some(block) => Ok(Json(serde_json::to_value(block).map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al serializar el bloque".to_string()))?)),
        None => Err((StatusCode::NOT_FOUND, format!("Bloque {} no encontrado", index)))
    }
}

async fn mine(
    State(blockchain): State<Arc<Mutex<Blockchain>>>
) -> Result<String, (StatusCode, String)> {
    let mut blockchain = blockchain.lock()
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error interno".to_string()))?;

    blockchain.minar();
    blockchain.guardar("blockchain.json").map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al guardar la cadena".to_string()))?;
    info!("Nuevo bloque minado");
    Ok("Bloque minado exitosamente\n".to_string())
}