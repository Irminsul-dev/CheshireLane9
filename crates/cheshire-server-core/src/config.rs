use std::net::SocketAddr;
use std::path::{Path, PathBuf};

use anyhow::{Context, Result};
use cheshire_server_proto::p10::Serverinfo;
use serde::Deserialize;

#[derive(Clone, Deserialize)]
pub struct Config {
    pub database_url: String,
    #[serde(default = "default_assets_dir")]
    pub assets_dir: PathBuf,
    pub sdk_http_addr: SocketAddr,
    pub sdk_https_addr: SocketAddr,
    #[serde(default = "default_sdk_ip")]
    pub sdk_ip: String,
    #[serde(default = "default_sdk_proxy_addr")]
    pub sdk_proxy_addr: SocketAddr,
    #[serde(default = "default_sdk_proxy_upstream_addr")]
    pub sdk_proxy_upstream_addr: SocketAddr,
    pub dispatch_addr: SocketAddr,
    pub gate_addr: SocketAddr,
    pub dispatch_ip: String,
    pub dispatch_port: u16,
    pub dispatch_version: Vec<String>,
    pub dispatch_servers: Vec<Serverinfo>,
    #[serde(default = "default_mitm_ca_cert_path")]
    pub mitm_ca_cert_path: PathBuf,
    #[serde(default = "default_mitm_ca_key_path")]
    pub mitm_ca_key_path: PathBuf,
    #[serde(default = "default_tls_cert_path")]
    pub tls_cert_path: PathBuf,
    #[serde(default = "default_tls_key_path")]
    pub tls_key_path: PathBuf,
}

fn default_sdk_ip() -> String {
    "127.0.0.1".to_string()
}

fn default_assets_dir() -> PathBuf {
    "assets".into()
}

fn default_sdk_proxy_addr() -> SocketAddr {
    "0.0.0.0:28080".parse().unwrap()
}

fn default_sdk_proxy_upstream_addr() -> SocketAddr {
    "127.0.0.1:21080".parse().unwrap()
}

fn default_mitm_ca_cert_path() -> PathBuf {
    "assets/ca/ca-cert.cer".into()
}

fn default_mitm_ca_key_path() -> PathBuf {
    "assets/ca/ca-key.pem".into()
}

fn default_tls_cert_path() -> PathBuf {
    "assets/tls/cert.pem".into()
}

fn default_tls_key_path() -> PathBuf {
    "assets/tls/key.pem".into()
}

impl Default for Config {
    fn default() -> Self {
        toml::from_str(DEFAULT_CONFIG).expect("default config must be valid")
    }
}

impl Config {
    pub fn load_or_create(path: impl AsRef<Path>) -> Result<Self> {
        let path = path.as_ref();
        match std::fs::read_to_string(path) {
            Ok(data) => toml::from_str(&data)
                .with_context(|| format!("parse configuration {}", path.display())),
            Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
                std::fs::write(path, DEFAULT_CONFIG)
                    .with_context(|| format!("write default configuration {}", path.display()))?;
                Ok(Self::default())
            }
            Err(err) => Err(err).with_context(|| format!("read configuration {}", path.display())),
        }
    }

    pub fn sdk_http_origin(&self) -> String {
        format!("http://{}:{}", self.sdk_ip, self.sdk_http_addr.port())
    }
}

pub const DEFAULT_CONFIG: &str = include_str!("config.default.toml");

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::*;

    #[test]
    fn load_or_create_writes_loopback_defaults() {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path = std::env::temp_dir().join(format!("cheshire-server-{id}.toml"));
        let config = Config::load_or_create(&path).unwrap();
        let data = std::fs::read_to_string(&path).unwrap();
        let _ = std::fs::remove_file(&path);

        assert_eq!(config.assets_dir, PathBuf::from("assets"));
        assert_eq!(config.dispatch_addr.ip().to_string(), "0.0.0.0");
        assert_eq!(config.sdk_ip, "127.0.0.1");
        assert_eq!(config.sdk_proxy_addr, default_sdk_proxy_addr());
        assert_eq!(
            config.sdk_proxy_upstream_addr,
            default_sdk_proxy_upstream_addr()
        );
        assert_eq!(config.dispatch_ip, "127.0.0.1");
        assert!(config.dispatch_servers.iter().all(|server| {
            server.ip == "127.0.0.1" && server.proxy_ip.as_deref() == Some("127.0.0.1")
        }));
        assert!(data.contains("ip = \"127.0.0.1\""));
        assert!(data.contains("sdk_proxy_addr = \"0.0.0.0:28080\""));
    }

    #[test]
    fn old_config_without_sdk_ip_uses_loopback() {
        let config: Config = toml::from_str(
            r#"
database_url = "sqlite://cheshire.sqlite"
sdk_http_addr = "0.0.0.0:21080"
sdk_https_addr = "0.0.0.0:21443"
dispatch_addr = "0.0.0.0:21180"
gate_addr = "0.0.0.0:21280"
dispatch_ip = "127.0.0.1"
dispatch_port = 21180
dispatch_version = []
dispatch_servers = []
tls_cert_path = "assets/tls/cert.pem"
tls_key_path = "assets/tls/key.pem"
"#,
        )
        .unwrap();

        assert_eq!(config.assets_dir, default_assets_dir());
        assert_eq!(config.sdk_ip, "127.0.0.1");
        assert_eq!(config.sdk_proxy_addr, default_sdk_proxy_addr());
        assert_eq!(
            config.sdk_proxy_upstream_addr,
            default_sdk_proxy_upstream_addr()
        );
        assert_eq!(config.mitm_ca_cert_path, default_mitm_ca_cert_path());
        assert_eq!(config.mitm_ca_key_path, default_mitm_ca_key_path());
    }
}
