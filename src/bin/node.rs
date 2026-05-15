use mini_blockchain::{node::Node, config::Config, error::NodeError};

#[tokio::main]
async fn main() -> Result<(), NodeError> {
    tracing_subscriber::fmt::init();
    Node::new(Config::from_env())?.run().await
}
