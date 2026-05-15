use mini_blockchain::{api, config::Config};

#[tokio::main]
async fn main() {
    api::serve(Config::from_env()).await;
}
