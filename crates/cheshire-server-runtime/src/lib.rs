use std::future::{pending, Future};

use anyhow::{anyhow, Context, Result};
use cheshire_server_core::{data, Database, PlayerRuntime};
use cheshire_server_services::{certificates, dispatch, gate, sdk, sdk_proxy};
use tokio::task::JoinSet;

pub use cheshire_server_core::Config;

pub struct Server {
    config: Config,
}

impl Server {
    pub fn new(config: Config) -> Self {
        Self { config }
    }

    pub fn config(&self) -> &Config {
        &self.config
    }

    pub async fn run(self) -> Result<()> {
        self.run_until_shutdown(pending()).await
    }

    pub async fn run_until_shutdown<F>(self, shutdown: F) -> Result<()>
    where
        F: Future<Output = ()>,
    {
        install_crypto_provider()?;
        certificates::ensure(&self.config).context("prepare server certificates")?;
        data::load_all(&self.config.assets_dir).context("load game data")?;

        let db = Database::connect(&self.config.database_url)
            .await
            .context("connect server database")?;
        let player_runtime = PlayerRuntime::new(db.clone(), self.config.sdk_http_origin());

        let mut services = JoinSet::new();
        let config = self.config;

        let dispatch_config = config.clone();
        services.spawn(async move {
            dispatch::serve(dispatch_config)
                .await
                .context("dispatch service")
        });

        let gate_addr = config.gate_addr;
        let gate_db = db.clone();
        services.spawn(async move {
            gate::serve(gate_addr, gate_db, player_runtime)
                .await
                .context("gate service")
        });

        let sdk_config = config.clone();
        services.spawn(async move { sdk::serve(sdk_config, db).await.context("SDK service") });

        services.spawn(async move { sdk_proxy::serve(config).await.context("SDK proxy service") });

        tokio::pin!(shutdown);
        let result = tokio::select! {
            service = services.join_next() => match service {
                Some(Ok(result)) => result,
                Some(Err(err)) => Err(anyhow!("server service task failed: {err}")),
                None => Err(anyhow!("server started without any services")),
            },
            _ = &mut shutdown => {
                tracing::info!("shutdown requested");
                Ok(())
            },
        };

        services.abort_all();
        while services.join_next().await.is_some() {}

        result
    }
}

fn install_crypto_provider() -> Result<()> {
    if rustls::crypto::CryptoProvider::get_default().is_none()
        && rustls::crypto::ring::default_provider()
            .install_default()
            .is_err()
        && rustls::crypto::CryptoProvider::get_default().is_none()
    {
        return Err(anyhow!("failed to install rustls crypto provider"));
    }

    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn accepts_programmatic_configuration() {
        let config = Config {
            database_url: "sqlite::memory:".to_string(),
            ..Default::default()
        };

        let server = Server::new(config);

        assert_eq!(server.config().database_url, "sqlite::memory:");
    }

    #[tokio::test]
    async fn starts_from_memory_config_and_obeys_shutdown() {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cheshire-runtime-{id}"));
        let config = Config {
            database_url: "sqlite::memory:".to_string(),
            assets_dir: std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../assets"),
            sdk_http_addr: "127.0.0.1:0".parse().unwrap(),
            sdk_https_addr: "127.0.0.1:0".parse().unwrap(),
            sdk_proxy_addr: "127.0.0.1:0".parse().unwrap(),
            dispatch_addr: "127.0.0.1:0".parse().unwrap(),
            gate_addr: "127.0.0.1:0".parse().unwrap(),
            mitm_ca_cert_path: root.join("ca/ca-cert.cer"),
            mitm_ca_key_path: root.join("ca/ca-key.pem"),
            tls_cert_path: root.join("tls/cert.pem"),
            tls_key_path: root.join("tls/key.pem"),
            ..Default::default()
        };

        Server::new(config)
            .run_until_shutdown(async {})
            .await
            .unwrap();

        let _ = std::fs::remove_dir_all(root);
    }
}
