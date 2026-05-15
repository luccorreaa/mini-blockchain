//! P2P gossip node using libp2p (mDNS discovery, GossipSub, request–response).
//!
//! On startup the node:
//! 1. Discovers peers on the local network via mDNS.
//! 2. On first connection to a peer, requests its full chain via request–response.
//! 3. Adopts the peer's chain if it is longer and valid (longest-chain rule).
//! 4. Subscribes to the `blocks` and `transactions` GossipSub topics to receive
//!    newly mined blocks and pending transactions propagated by other nodes.
//!
//! Interactive stdin commands:
//! - `tx <from_hex> <to_hex> <amount>` — broadcast a new transaction.
//! - `mine` — mine the next block and broadcast it.

use libp2p::{
    gossipsub, mdns, noise, tcp, yamux,
    swarm::{NetworkBehaviour, SwarmEvent},
    SwarmBuilder,
    request_response::{self, ProtocolSupport},
    StreamProtocol,
};
use libp2p::futures::StreamExt;
use tokio::io::{self, AsyncBufReadExt};
use serde::{Serialize, Deserialize};
use mini_blockchain::chain::blockchain::Blockchain;
use mini_blockchain::chain::block::Block;
use mini_blockchain::crypto::transaction::Transaction;
use mini_blockchain::types::PublicKey;
use std::collections::HashSet;
use std::time::Duration;
use tracing::{error, info, warn};

/// Combined libp2p network behaviour for the blockchain node.
#[derive(NetworkBehaviour)]
struct NodeBehaviour {
    gossipsub: gossipsub::Behaviour,
    mdns: mdns::tokio::Behaviour,
    request_response: libp2p::request_response::cbor::Behaviour<ChainRequest, ChainResponse>,
}

/// Request message sent to a peer to obtain its full chain.
#[derive(Debug, Serialize, Deserialize)]
struct ChainRequest;

/// Response carrying a peer's full blockchain.
#[derive(Debug, Serialize, Deserialize)]
struct ChainResponse {
    chain: Blockchain,
}

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let mut swarm = SwarmBuilder::with_new_identity()
        .with_tokio()
        .with_tcp(
            tcp::Config::default(),
            noise::Config::new,
            yamux::Config::default,
        )?
        .with_behaviour(|key| {
            let gossipsub_config = gossipsub::ConfigBuilder::default().build()?;
            let gossipsub = gossipsub::Behaviour::new(
                gossipsub::MessageAuthenticity::Signed(key.clone()),
                gossipsub_config,
            )?;

            let request_response = request_response::cbor::Behaviour::new(
                [(StreamProtocol::new("/blockchain/1.0.0"), ProtocolSupport::Full)],
                request_response::Config::default(),
            );

            let mdns = mdns::tokio::Behaviour::new(
                mdns::Config::default(),
                key.public().to_peer_id(),
            )?;

            Ok(NodeBehaviour { gossipsub, mdns, request_response })
        })?
        .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
        .build();

    let mut blockchain = Blockchain::load("blockchain.json")
        .unwrap_or_else(|_| Blockchain::new());

    let mut synced_peers: HashSet<libp2p::PeerId> = HashSet::new();

    swarm.listen_on("/ip4/0.0.0.0/tcp/0".parse()?)?;

    let block_topic = gossipsub::IdentTopic::new("blocks");
    let tx_topic = gossipsub::IdentTopic::new("transactions");
    swarm.behaviour_mut().gossipsub.subscribe(&block_topic)?;
    swarm.behaviour_mut().gossipsub.subscribe(&tx_topic)?;

    // Precompute topic hashes once to avoid repeated allocations on every gossip message.
    let block_topic_hash = block_topic.hash();
    let tx_topic_hash = tx_topic.hash();

    let mut stdin = io::BufReader::new(io::stdin()).lines();

    loop {
        tokio::select! {
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                    for (peer_id, addr) in peers {
                        swarm.add_peer_address(peer_id, addr);
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        if let Err(e) = swarm.dial(peer_id) {
                            warn!("Failed to dial {peer_id}: {e:?}");
                        } else {
                            info!("Peer discovered, connecting: {peer_id}");
                        }
                    }
                }
                SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(mdns::Event::Expired(peers))) => {
                    for (peer_id, _addr) in peers {
                        swarm.behaviour_mut().gossipsub.remove_explicit_peer(&peer_id);
                        info!("Peer expired: {peer_id}");
                    }
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    if synced_peers.insert(peer_id) {
                        swarm.behaviour_mut().request_response.send_request(&peer_id, ChainRequest);
                        info!("Connection established with: {peer_id}");
                    }
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    // Remove so that a future reconnection triggers a fresh chain sync.
                    synced_peers.remove(&peer_id);
                    info!("Connection closed with: {peer_id}");
                }
                SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source,
                    message,
                    ..
                })) => {
                    if message.topic == block_topic_hash {
                        if let Ok(block) = serde_json::from_slice::<Block>(&message.data) {
                            // Validate hash integrity and chain linkage before appending.
                            let tip_hash = blockchain.chain().last().map(|b| b.hash().clone());
                            let valid = block.hash() == &block.compute_hash()
                                && tip_hash.as_ref() == Some(block.prev_hash());
                            if valid {
                                blockchain.push_block(block);
                                blockchain.save("blockchain.json").ok();
                                info!("Block received and appended from {propagation_source}");
                            } else {
                                warn!("Block from {propagation_source} rejected (invalid hash or wrong prev_hash)");
                            }
                        }
                    } else if message.topic == tx_topic_hash {
                        if let Ok(tx) = serde_json::from_slice::<Transaction>(&message.data) {
                            if let Err(e) = blockchain.add_transaction(tx) {
                                warn!("Transaction from {propagation_source} rejected: {e}");
                            } else {
                                info!("Transaction received from {propagation_source}");
                            }
                        }
                    }
                }
                SwarmEvent::Behaviour(NodeBehaviourEvent::RequestResponse(
                    request_response::Event::Message { peer, message }
                )) => match message {
                    request_response::Message::Request { channel, .. } => {
                        let response = ChainResponse { chain: blockchain.clone() };
                        swarm.behaviour_mut().request_response.send_response(channel, response).ok();
                        info!("Chain sent to {peer}");
                    }
                    request_response::Message::Response { response, .. } => {
                        let received_chain = response.chain;
                        if received_chain.chain().len() > blockchain.chain().len()
                            && received_chain.validate()
                        {
                            info!(
                                "Longer chain adopted from {peer}: {} blocks",
                                received_chain.chain().len()
                            );
                            blockchain = received_chain;
                            blockchain.save("blockchain.json").ok();
                        } else {
                            info!("Chain from {peer} discarded (not longer or invalid)");
                        }
                    }
                },
                SwarmEvent::Behaviour(NodeBehaviourEvent::RequestResponse(
                    request_response::Event::OutboundFailure { peer, error, .. }
                )) => {
                    error!("Request error to {peer}: {error:?}");
                }
                _ => {}
            },
            line = stdin.next_line() => {
                match line {
                    Ok(Some(line)) => {
                        if line.starts_with("tx ") {
                            let parts: Vec<&str> = line.splitn(4, ' ').collect();
                            if parts.len() == 4 {
                                if let Ok(amount) = parts[3].trim().parse::<u64>() {
                                    let from = hex::decode(parts[1]).ok()
                                        .and_then(|b| b.try_into().ok());
                                    let to = hex::decode(parts[2]).ok()
                                        .and_then(|b| b.try_into().ok());
                                    if let (Some(from), Some(to)) = (from, to) {
                                        let tx = Transaction::new(PublicKey(from), PublicKey(to), amount);
                                        if let Ok(bytes) = serde_json::to_vec(&tx) {
                                            swarm.behaviour_mut().gossipsub.publish(tx_topic.clone(), bytes).ok();
                                        }
                                        // Also add to the local mempool so this node includes it when mining.
                                        if let Err(e) = blockchain.add_transaction(tx) {
                                            warn!("Transaction rejected locally: {e}");
                                        } else {
                                            info!("Transaction propagated");
                                        }
                                    }
                                }
                            }
                        } else if line.trim() == "mine" {
                            blockchain.mine();
                            if let Some(block) = blockchain.chain().last() {
                                if let Ok(bytes) = serde_json::to_vec(block) {
                                    swarm.behaviour_mut().gossipsub.publish(block_topic.clone(), bytes).ok();
                                    info!("Block mined and propagated");
                                }
                            }
                            blockchain.save("blockchain.json").ok();
                        }
                    }
                    Ok(None) => break, // stdin closed (EOF)
                    Err(e) => {
                        error!("Stdin read error: {e}");
                        break;
                    }
                }
            }
        }
    }

    Ok(())
}
