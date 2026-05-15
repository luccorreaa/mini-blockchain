pub mod handlers;

use std::sync::Arc;
use tokio::sync::RwLock;
use axum::{Router, routing::{get, post}};
use tracing_subscriber::EnvFilter;
use crate::chain::blockchain::Blockchain;
use crate::config::{Config, API_BIND_ADDR};

pub struct ApiState {
    pub blockchain: RwLock<Blockchain>,
    pub config:     Config,
}

pub type AppState = Arc<ApiState>;

pub async fn serve(config: Config) {
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info")),
        )
        .init();

    let blockchain = Blockchain::load(config.chain_path.to_str().unwrap())
        .unwrap_or_default();

    let state: AppState = Arc::new(ApiState {
        blockchain: RwLock::new(blockchain),
        config,
    });

    let app = Router::new()
        .route("/chain",        get(handlers::chain::get_chain))
        .route("/validate",     get(handlers::chain::validate))
        .route("/block/:index", get(handlers::chain::get_block))
        .route("/wallet",       post(handlers::wallet::new_wallet))
        .route("/transaction",  post(handlers::transaction::add_to_mempool))
        .route("/mine",         post(handlers::mining::mine))
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(API_BIND_ADDR).await.unwrap();
    tracing::info!(addr = "http://localhost:3000", "API started");
    axum::serve(listener, app).await.unwrap();
}
