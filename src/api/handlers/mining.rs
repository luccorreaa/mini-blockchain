use axum::{extract::State, http::StatusCode};
use crate::api::AppState;
use crate::chain::block::Block;
use crate::crypto::wallet::Wallet;

pub async fn mine(State(s): State<AppState>) -> Result<String, (StatusCode, String)> {
    let chain_path  = s.config.chain_path.to_str().unwrap().to_string();
    let wallet_path = s.config.wallet_path.to_str().unwrap().to_string();
    let reward      = s.config.coinbase_reward;

    let (index, prev_hash, txs, difficulty) = {
        let mut bc = s.blockchain.write().await;
        if let Ok(wallet) = Wallet::load_encrypted(&wallet_path, &s.config.wallet_password) {
            bc.add_coinbase(wallet.pubkey, reward);
        }
        let (tip_index, tip_hash) = bc.tip()
            .ok_or((StatusCode::INTERNAL_SERVER_ERROR, "Empty chain".to_string()))?;
        let txs = bc.take_mempool();
        let diff = bc.difficulty();
        (tip_index + 1, tip_hash, txs, diff)
    };

    let block = tokio::task::spawn_blocking(move || {
        let mut b = Block::new(index, txs, &prev_hash);
        b.mine(difficulty);
        b
    }).await.map_err(|_| (StatusCode::INTERNAL_SERVER_ERROR, "Mining thread error".to_string()))?;

    let mut bc = s.blockchain.write().await;
    bc.push_block(block);
    bc.save(&chain_path).map_err(|e| (StatusCode::INTERNAL_SERVER_ERROR, e.to_string()))?;

    tracing::info!("New block mined");
    Ok("Block mined successfully\n".to_string())
}
