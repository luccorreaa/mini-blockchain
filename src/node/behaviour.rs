use libp2p::{gossipsub, mdns, request_response, swarm::NetworkBehaviour};
use serde::{Serialize, Deserialize};
use crate::chain::blockchain::Blockchain;

#[derive(NetworkBehaviour)]
pub struct NodeBehaviour {
    pub gossipsub:        gossipsub::Behaviour,
    pub mdns:             mdns::tokio::Behaviour,
    pub request_response: request_response::cbor::Behaviour<ChainRequest, ChainResponse>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainRequest;

#[derive(Debug, Serialize, Deserialize)]
pub struct ChainResponse {
    pub chain: Blockchain,
}
