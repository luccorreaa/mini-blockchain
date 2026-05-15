use tracing::{info, warn};
use crate::node::NodeState;
use crate::crypto::transaction::Transaction;
use crate::types::PublicKey;

pub fn handle_stdin_command(line: &str, state: &mut NodeState) {
    if line.starts_with("tx ") {
        handle_tx_command(line, state);
    } else if line.trim() == "mine" {
        handle_mine_command(state);
    }
}

fn handle_tx_command(line: &str, state: &mut NodeState) {
    let parts: Vec<&str> = line.splitn(4, ' ').collect();
    if parts.len() != 4 { return; }
    let amount = match parts[3].trim().parse::<u64>() {
        Ok(a) => a,
        Err(_) => return,
    };
    let from = hex::decode(parts[1]).ok().and_then(|b| PublicKey::try_from(b).ok());
    let to   = hex::decode(parts[2]).ok().and_then(|b| PublicKey::try_from(b).ok());

    if let (Some(from), Some(to)) = (from, to) {
        let tx = Transaction::new(from, to, amount);
        if let Ok(bytes) = serde_json::to_vec(&tx) {
            state.swarm.behaviour_mut().gossipsub
                .publish(state.tx_topic.clone(), bytes).ok();
        }
        match state.blockchain.add_transaction(tx) {
            Ok(_)  => info!("Transaction propagated"),
            Err(e) => warn!("Transaction rejected locally: {e}"),
        }
    }
}

fn handle_mine_command(state: &mut NodeState) {
    state.blockchain.mine();
    if let Some(block) = state.blockchain.chain().last()
        && let Ok(bytes) = serde_json::to_vec(block) {
            state.swarm.behaviour_mut().gossipsub
                .publish(state.block_topic.clone(), bytes).ok();
            info!("Block mined and propagated");
        }
    state.blockchain.save(&state.chain_path).ok();
}
