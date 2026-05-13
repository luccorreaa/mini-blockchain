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
use mini_blockchain::blockchain::Blockchain;
use std::collections::HashSet;
use std::time::Duration;

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

    let mut stdin = io::BufReader::new(io::stdin()).lines();

    loop {
        tokio::select! {
            event = swarm.select_next_some() => match event {
                SwarmEvent::Behaviour(NodeBehaviourEvent::Mdns(mdns::Event::Discovered(peers))) => {
                    for (peer_id, addr) in peers {
                        swarm.add_peer_address(peer_id, addr);
                        swarm.behaviour_mut().gossipsub.add_explicit_peer(&peer_id);
                        if let Err(e) = swarm.dial(peer_id) {
                            println!("Failed to dial {}: {:?}", peer_id, e);
                        } else {
                            println!("Peer discovered, connecting: {}", peer_id);
                        }
                    }
                }
                SwarmEvent::ConnectionEstablished { peer_id, .. } => {
                    if synced_peers.insert(peer_id) {
                        swarm.behaviour_mut().request_response.send_request(&peer_id, ChainRequest);
                        println!("Connection established with: {}", peer_id);
                    }
                }
                SwarmEvent::ConnectionClosed { peer_id, .. } => {
                    println!("Connection closed with: {}", peer_id);
                }
                SwarmEvent::Behaviour(NodeBehaviourEvent::Gossipsub(gossipsub::Event::Message {
                    propagation_source,
                    message,
                    ..
                })) => {
                    let topic = message.topic.to_string();
                    if topic == block_topic.hash().to_string() {
                        if let Ok(block) = serde_json::from_slice::<mini_blockchain::block::Block>(&message.data) {
                            blockchain.push_block(block);
                            blockchain.save("blockchain.json").ok();
                            println!("Block received from {}", propagation_source);
                        }
                    } else if topic == tx_topic.hash().to_string() {
                        if let Ok(tx) = serde_json::from_slice::<mini_blockchain::transactions::Transaction>(&message.data) {
                            blockchain.add_transaction(tx).ok();
                            println!("Transaction received from {}", propagation_source);
                        }
                    }
                }
                SwarmEvent::Behaviour(NodeBehaviourEvent::RequestResponse(
                    request_response::Event::Message { peer, message }
                )) => match message {
                    request_response::Message::Request { channel, .. } => {
                        let response = ChainResponse { chain: blockchain.clone() };
                        swarm.behaviour_mut().request_response.send_response(channel, response).ok();
                        println!("Chain sent to {}", peer);
                    }
                    request_response::Message::Response { response, .. } => {
                        let received_chain = response.chain;
                        if received_chain.chain().len() > blockchain.chain().len()
                            && received_chain.validate()
                        {
                            println!(
                                "Longer chain adopted from {}: {} blocks",
                                peer,
                                received_chain.chain().len()
                            );
                            blockchain = received_chain;
                            blockchain.save("blockchain.json").ok();
                        } else {
                            println!("Chain from {} discarded (not longer or invalid)", peer);
                        }
                    }
                },
                SwarmEvent::Behaviour(NodeBehaviourEvent::RequestResponse(
                    request_response::Event::OutboundFailure { peer, error, .. }
                )) => {
                    println!("Request error to {}: {:?}", peer, error);
                }
                _ => {}
            },
            line = stdin.next_line() => {
                if let Ok(Some(line)) = line {
                    if line.starts_with("tx ") {
                        let parts: Vec<&str> = line.splitn(4, ' ').collect();
                        if parts.len() == 4 {
                            if let Ok(amount) = parts[3].trim().parse::<u64>() {
                                let from = hex::decode(parts[1]).ok()
                                    .and_then(|b| b.try_into().ok());
                                let to = hex::decode(parts[2]).ok()
                                    .and_then(|b| b.try_into().ok());
                                if let (Some(from), Some(to)) = (from, to) {
                                    let tx = mini_blockchain::transactions::Transaction::new(from, to, amount);
                                    if let Ok(bytes) = serde_json::to_vec(&tx) {
                                        swarm.behaviour_mut().gossipsub.publish(tx_topic.clone(), bytes).ok();
                                        println!("Transaction propagated");
                                    }
                                }
                            }
                        }
                    } else if line.trim() == "mine" {
                        blockchain.mine();
                        if let Some(block) = blockchain.chain().last() {
                            if let Ok(bytes) = serde_json::to_vec(block) {
                                swarm.behaviour_mut().gossipsub.publish(block_topic.clone(), bytes).ok();
                                println!("Block mined and propagated");
                            }
                        }
                        blockchain.save("blockchain.json").ok();
                    }
                }
            }
        }
    }
}
