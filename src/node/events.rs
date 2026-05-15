use libp2p::{gossipsub, mdns, request_response, swarm::SwarmEvent};
use tracing::{error, info, warn};
use crate::node::behaviour::{NodeBehaviourEvent, ChainRequest, ChainResponse};
use crate::node::NodeState;

pub fn handle_swarm_event(
    event: SwarmEvent<NodeBehaviourEvent>,
    state: &mut NodeState,
) {
    match event {
        SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
            for (peer_id, addr) in peers {
                state.swarm.add_peer_address(peer_id, addr);
                state.swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                if let Err(e) = state.swarm.dial(peer_id) {
                    warn!("Failed to dial {peer_id}: {e:?}");
                } else {
                    info!("Peer discovered, connecting: {peer_id}");
                }
            }
        }
        SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
            for (peer_id, _) in peers {
                state.swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                info!("Peer expired: {peer_id}");
            }
        }
        SwarmEvent::ConnectionEstablished { peer_id, .. }
            if state.synced_peers.insert(peer_id) => {
                state.swarm.behaviour_mut().request_response
                    .send_request(&peer_id, ChainRequest);
                info!("Connection established with: {peer_id}");
            }
        SwarmEvent::ConnectionClosed { peer_id, .. } => {
            state.synced_peers.remove(&peer_id);
            info!("Connection closed with: {peer_id}");
        }
        SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(
            gossipsub::Event::Message { propagation_source, message, .. }
        )) => {
            handle_gossip_message(propagation_source, message, state);
        }
        SwarmEvent::Behaviour(NodeBehaviourEvent::RequestResponse(
            request_response::Event::Message { peer, message }
        )) => {
            handle_request_response(peer, message, state);
        }
        SwarmEvent::Behaviour(NodeBehaviourEvent::RequestResponse(
            request_response::Event::OutboundFailure { peer, error, .. }
        )) => {
            error!("Request error to {peer}: {error:?}");
        }
        _ => {}
    }
}

fn handle_gossip_message(
    source: libp2p::PeerId,
    message: gossipsub::Message,
    state: &mut NodeState,
) {
    let block_hash = state.block_topic.hash();
    let tx_hash    = state.tx_topic.hash();

    if message.topic == block_hash {
        if let Ok(block) = serde_json::from_slice::<crate::chain::block::Block>(&message.data) {
            let tip_hash = state.blockchain.chain().last().map(|b| b.hash().clone());
            let valid = block.hash() == &block.compute_hash()
                && tip_hash.as_ref() == Some(block.prev_hash());
            if valid {
                state.blockchain.push_block(block);
                state.blockchain.save(&state.chain_path).ok();
                info!("Block received and appended from {source}");
            } else {
                warn!("Block from {source} rejected (invalid hash or wrong prev_hash)");
            }
        }
    } else if message.topic == tx_hash
        && let Ok(tx) = serde_json::from_slice::<crate::crypto::transaction::Transaction>(&message.data) {
            match state.blockchain.add_transaction(tx) {
                Ok(_)  => info!("Transaction received from {source}"),
                Err(e) => warn!("Transaction from {source} rejected: {e}"),
            }
        }
}

fn handle_request_response(
    peer: libp2p::PeerId,
    message: request_response::Message<ChainRequest, ChainResponse>,
    state: &mut NodeState,
) {
    match message {
        request_response::Message::Request { channel, .. } => {
            let response = ChainResponse { chain: state.blockchain.clone() };
            state.swarm.behaviour_mut().request_response
                .send_response(channel, response).ok();
            info!("Chain sent to {peer}");
        }
        request_response::Message::Response { response, .. } => {
            let received = response.chain;
            if received.chain().len() > state.blockchain.chain().len() && received.validate() {
                info!("Longer chain adopted from {peer}: {} blocks", received.chain().len());
                state.blockchain = received;
                state.blockchain.save(&state.chain_path).ok();
            } else {
                info!("Chain from {peer} discarded (not longer or invalid)");
            }
        }
    }
}
