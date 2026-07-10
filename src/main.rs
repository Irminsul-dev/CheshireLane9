mod certificates;
mod config;
mod crypto;
mod data;
mod database;
mod dispatch;
mod game;
mod gate;
mod packet;
mod sdk;
mod sdk_proxy;
mod time;

use anyhow::Result;
use config::CONFIG;
use database::Database;
use game::PlayerRuntime;
use tracing_subscriber::EnvFilter;

#[tokio::main]
async fn main() -> Result<()> {
    init_tracing();
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install rustls crypto provider"))?;

    certificates::ensure(&CONFIG)?;
    data::load_all()?;
    let db = Database::connect(&CONFIG.database_url).await?;
    let runtime = PlayerRuntime::new(db.clone());

    let sdk_db = db.clone();
    let dispatch_task = tokio::spawn(dispatch::serve());
    let gate_task = tokio::spawn(gate::serve(db, runtime));
    let sdk_task = tokio::spawn(sdk::serve(sdk_db));
    let sdk_proxy_task = tokio::spawn(sdk_proxy::serve());

    tokio::select! {
        result = dispatch_task => result??,
        result = gate_task => result??,
        result = sdk_task => result??,
        result = sdk_proxy_task => result??,
        _ = tokio::signal::ctrl_c() => tracing::info!("shutdown requested"),
    }

    Ok(())
}

fn init_tracing() {
    let filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info,hudsucker=off"));
    tracing_subscriber::fmt().with_env_filter(filter).init();
}
