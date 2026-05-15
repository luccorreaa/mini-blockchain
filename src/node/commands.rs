use tracing::{info, warn};
use crate::node::NodeState;

pub fn handle_stdin_command(line: &str, state: &mut NodeState) {
    match line.trim() {
        "mine" => handle_mine_command(state),
        "help" => handle_help_command(),
        other  => warn!("Unknown command: '{other}'. Type 'help' for available commands."),
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

fn handle_help_command() {
    println!("Available commands:");
    println!("  mine    mine a block and broadcast it to peers");
    println!("  help    show this message");
}
