pub mod behaviour;
pub mod commands;
pub mod events;

use std::collections::HashSet;
use std::time::Duration;
use libp2p::{
    gossipsub, mdns, noise, tcp, yamux,
    SwarmBuilder,
    request_response::{self, ProtocolSupport},
    StreamProtocol, PeerId,
};
use libp2p::futures::StreamExt;
use tokio::io::{self, AsyncBufReadExt};
use tracing::error;
use crate::chain::blockchain::Blockchain;
use crate::config::Config;
use crate::error::{NodeError, NodeResult};
use behaviour::NodeBehaviour;

pub struct NodeState {
    pub swarm:        libp2p::Swarm<NodeBehaviour>,
    pub blockchain:   Blockchain,
    pub synced_peers: HashSet<PeerId>,
    pub block_topic:  gossipsub::IdentTopic,
    pub tx_topic:     gossipsub::IdentTopic,
    pub chain_path:   String,
}

pub struct Node {
    state: NodeState,
}

impl Node {
    pub fn new(config: Config) -> NodeResult<Self> {
        let swarm = SwarmBuilder::with_new_identity()
            .with_tokio()
            .with_tcp(
                tcp::Config::default(),
                noise::Config::new,
                yamux::Config::default,
            )
            .map_err(|e| NodeError::Transport(e.to_string()))?
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
            })
            .map_err(|e| NodeError::Transport(e.to_string()))?
            .with_swarm_config(|c| c.with_idle_connection_timeout(Duration::from_secs(60)))
            .build();

        let chain_path = config.chain_path.to_str().unwrap().to_string();
        let blockchain = Blockchain::load(&chain_path).unwrap_or_default();
        let block_topic = gossipsub::IdentTopic::new("blocks");
        let tx_topic    = gossipsub::IdentTopic::new("transactions");

        Ok(Self {
            state: NodeState {
                swarm,
                blockchain,
                synced_peers: HashSet::new(),
                block_topic,
                tx_topic,
                chain_path,
            },
        })
    }

    pub async fn run(mut self) -> NodeResult<()> {
        self.state.swarm
            .listen_on("/ip4/0.0.0.0/tcp/0".parse().unwrap())
            .unwrap();

        {
            let s = &mut self.state;
            s.swarm.behaviour_mut().gossipsub.subscribe(&s.block_topic).unwrap();
            s.swarm.behaviour_mut().gossipsub.subscribe(&s.tx_topic).unwrap();
        }

        let mut stdin = io::BufReader::new(io::stdin()).lines();

        loop {
            tokio::select! {
                event = self.state.swarm.select_next_some() => {
                    events::handle_swarm_event(event, &mut self.state);
                }
                line = stdin.next_line() => {
                    match line {
                        Ok(Some(line)) => commands::handle_stdin_command(&line, &mut self.state),
                        Ok(None)       => break,
                        Err(e)         => { error!("Stdin read error: {e}"); break; }
                    }
                }
            }
        }
        Ok(())
    }
}
