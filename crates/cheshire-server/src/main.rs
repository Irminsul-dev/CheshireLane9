use anyhow::Result;
use cheshire_server_runtime::{Config, Server};
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();

    let config = Config::load_or_create("config.toml")?;
    Server::new(config)
        .run_until_shutdown(async {
            let _ = tokio::signal::ctrl_c().await;
        })
        .await
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,hudsucker=off"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
