mod config;
mod crypto;
mod data;
mod database;
mod dispatch;
mod game;
mod gate;
mod packet;
mod sdk;
mod time;

use anyhow::Result;
use config::CONFIG;
use database::Database;
use game::PlayerRuntime;

#[tokio::main]
async fn main() -> Result<()> {
    tracing_subscriber::fmt::init();
    rustls::crypto::ring::default_provider()
        .install_default()
        .map_err(|_| anyhow::anyhow!("failed to install rustls crypto provider"))?;

    data::load_all()?;
    let db = Database::connect(&CONFIG.database_url).await?;
    let runtime = PlayerRuntime::new(db.clone());

    let sdk_db = db.clone();
    let dispatch_task = tokio::spawn(dispatch::serve());
    let gate_task = tokio::spawn(gate::serve(db, runtime));
    let sdk_task = tokio::spawn(sdk::serve(sdk_db));

    tokio::select! {
        result = dispatch_task => result??,
        result = gate_task => result??,
        result = sdk_task => result??,
        _ = tokio::signal::ctrl_c() => tracing::info!("shutdown requested"),
    }

    Ok(())
}
