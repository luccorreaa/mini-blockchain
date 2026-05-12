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

#[derive(Deserialize)]
struct SendPayload {
    from: String,
    to: String,
    amount: u64,
}

fn wallet_password() -> String {
    std::env::var("WALLET_PASSWORD").unwrap_or_else(|_| "dev_password_change_me".to_string())
}

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt()
        .with_env_filter(EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")))
        .init();

    let blockchain = Blockchain::cargar("blockchain.json")
        .unwrap_or_else(|_| Blockchain::new_blockchain());
    let state: AppState = Arc::new(RwLock::new(blockchain));

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
    State(blockchain): State<AppState>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let bc = blockchain.read().await;
    Ok(Json(serde_json::to_value(&*bc)
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al serializar".to_string()))?))
}

async fn wallet() -> Result<String, (StatusCode, String)> {
    if std::path::Path::new("wallet.json").exists() {
        return Err((
            StatusCode::CONFLICT,
            "Ya existe una wallet. Eliminá wallet.json antes de generar una nueva.".to_string(),
        ));
    }
    let mut secret = [0u8; 32];
    OsRng.fill_bytes(&mut secret);
    let signing_key = SigningKey::from_bytes(&secret);
    let pubkey = signing_key.verifying_key().to_bytes();
    let pubkey_hex = hex::encode(pubkey);
    let w = Wallet::new(secret, pubkey);
    w.guardar_cifrado("wallet.json", &wallet_password())
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al guardar la wallet".to_string()))?;
    info!(pubkey = %pubkey_hex, "Nueva wallet generada");
    Ok(pubkey_hex)
}

async fn add_to_mempool(
    State(blockchain): State<AppState>,
    Json(payload): Json<SendPayload>,
) -> Result<String, (StatusCode, String)> {
    let from: [u8; 32] = hex::decode(&payload.from)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Clave 'from' inválida".to_string()))?
        .try_into()
        .map_err(|_| (StatusCode::BAD_REQUEST, "'from' debe ser 32 bytes".to_string()))?;

    let to: [u8; 32] = hex::decode(&payload.to)
        .map_err(|_| (StatusCode::BAD_REQUEST, "Clave 'to' inválida".to_string()))?
        .try_into()
        .map_err(|_| (StatusCode::BAD_REQUEST, "'to' debe ser 32 bytes".to_string()))?;

    let wallet = Wallet::cargar_cifrado("wallet.json", &wallet_password())
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al cargar la wallet".to_string()))?;
    let signing_key = SigningKey::from_bytes(&wallet.secret);

    let mut tx = Transaction::new(from, to, payload.amount);
    tx.firmar(&signing_key);

    let mut bc = blockchain.write().await;
    bc.add_transaction(tx)
        .map_err(|e| (StatusCode::BAD_REQUEST, e))?;

    info!(from = %payload.from, to = %payload.to, amount = payload.amount, "Transacción en mempool");
    Ok("Transacción enviada".to_string())
}

async fn validar(
    State(blockchain): State<AppState>,
) -> Result<String, (StatusCode, String)> {
    let bc = blockchain.read().await;
    let resultado = bc.validar();
    info!(valida = resultado, "Validación ejecutada");
    Ok(format!("La cadena de bloques es válida: {}", resultado))
}

async fn get_block(
    State(blockchain): State<AppState>,
    Path(index): Path<u32>,
) -> Result<Json<Value>, (StatusCode, String)> {
    let bc = blockchain.read().await;
    match bc.cadena().iter().find(|b| b.index() == index) {
        Some(block) => Ok(Json(serde_json::to_value(block)
            .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al serializar".to_string()))?)),
        None => Err((StatusCode::NOT_FOUND, format!("Bloque {} no encontrado", index))),
    }
}

async fn mine(
    State(blockchain): State<AppState>,
) -> Result<String, (StatusCode, String)> {
    // Fase 1: extraer datos y soltar el write lock inmediatamente
    let (index, prev_hash, txs, difficulty) = {
        let mut bc = blockchain.write().await;

        // Agregar coinbase para el minero si hay wallet disponible
        if let Ok(wallet) = Wallet::cargar_cifrado("wallet.json", &wallet_password()) {
            bc.add_coinbase(wallet.pubkey, 50);
        }

        let (tip_index, tip_hash) = bc.tip()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Cadena vacía".to_string()))?;
        let txs = bc.take_mempool();
        let difficulty = bc.difficulty();
        (tip_index + 1, tip_hash, txs, difficulty)
    }; // write lock liberado aquí

    // Fase 2: minar sin mantener ningún lock (Proof of Work puede tardar)
    let block = tokio::task::spawn_blocking(move || {
        let mut b = Block::new(index, txs, &prev_hash);
        b.minar(difficulty);
        b
    })
    .await
    .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error en el hilo de minado".to_string()))?;

    // Fase 3: agregar el bloque minado y guardar
    let mut bc = blockchain.write().await;
    bc.push_block(block);
    bc.guardar("blockchain.json")
        .map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Error al guardar".to_string()))?;

    info!("Nuevo bloque minado");
    Ok("Bloque minado exitosamente\n".to_string())
}
