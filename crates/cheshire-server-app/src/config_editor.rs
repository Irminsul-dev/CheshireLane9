use std::net::{Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use anyhow::{bail, Context, Result};
use cheshire_server_core::Config;
use cheshire_server_proto::p10::Serverinfo;

use crate::AppWindow;

#[derive(Clone, Debug)]
struct FormValues {
    database_url: String,
    sdk_http_addr: String,
    sdk_https_addr: String,
    sdk_ip: String,
    sdk_proxy_addr: String,
    sdk_proxy_upstream_addr: String,
    dispatch_addr: String,
    gate_addr: String,
    dispatch_ip: String,
    dispatch_port: String,
    mitm_ca_cert_path: String,
    mitm_ca_key_path: String,
    tls_cert_path: String,
    tls_key_path: String,
    dispatch_version_azhash: String,
    dispatch_version_cvhash: String,
    dispatch_version_l2dhash: String,
    dispatch_version_pichash: String,
    dispatch_version_bgmhash: String,
    dispatch_version_paintinghash: String,
    dispatch_version_mangahash: String,
    dispatch_version_cipherhash: String,
    dispatch_version_dormhash: String,
    dispatch_server_ip: String,
    dispatch_server_port: String,
    dispatch_server_name: String,
}

impl FormValues {
    fn from_config(config: &Config) -> Result<Self> {
        let versions = config
            .dispatch_version
            .iter()
            .filter(|version| !version.starts_with("dTag-"))
            .collect::<Vec<_>>();
        if versions.len() != 9 {
            bail!(
                "dispatch_version must contain exactly nine hash entries plus the automatic dTag entry"
            );
        }
        let server = config
            .dispatch_servers
            .first()
            .context("dispatch_servers must contain one server entry")?;

        Ok(Self {
            database_url: config.database_url.clone(),
            sdk_http_addr: config.sdk_http_addr.to_string(),
            sdk_https_addr: config.sdk_https_addr.to_string(),
            sdk_ip: config.sdk_ip.clone(),
            sdk_proxy_addr: config.sdk_proxy_addr.to_string(),
            sdk_proxy_upstream_addr: config.sdk_proxy_upstream_addr.to_string(),
            dispatch_addr: config.dispatch_addr.to_string(),
            gate_addr: config.gate_addr.to_string(),
            dispatch_ip: config.dispatch_ip.clone(),
            dispatch_port: config.dispatch_port.to_string(),
            mitm_ca_cert_path: config.mitm_ca_cert_path.display().to_string(),
            mitm_ca_key_path: config.mitm_ca_key_path.display().to_string(),
            tls_cert_path: config.tls_cert_path.display().to_string(),
            tls_key_path: config.tls_key_path.display().to_string(),
            dispatch_version_azhash: versions[0].clone(),
            dispatch_version_cvhash: versions[1].clone(),
            dispatch_version_l2dhash: versions[2].clone(),
            dispatch_version_pichash: versions[3].clone(),
            dispatch_version_bgmhash: versions[4].clone(),
            dispatch_version_paintinghash: versions[5].clone(),
            dispatch_version_mangahash: versions[6].clone(),
            dispatch_version_cipherhash: versions[7].clone(),
            dispatch_version_dormhash: versions[8].clone(),
            dispatch_server_ip: server.ip.clone(),
            dispatch_server_port: server.port.to_string(),
            dispatch_server_name: server.name.clone(),
        })
    }

    fn from_ui(ui: &AppWindow) -> Self {
        Self {
            database_url: ui.get_database_url().to_string(),
            sdk_http_addr: ui.get_sdk_http_addr().to_string(),
            sdk_https_addr: ui.get_sdk_https_addr().to_string(),
            sdk_ip: ui.get_sdk_ip().to_string(),
            sdk_proxy_addr: ui.get_sdk_proxy_addr().to_string(),
            sdk_proxy_upstream_addr: ui.get_sdk_proxy_upstream_addr().to_string(),
            dispatch_addr: ui.get_dispatch_addr().to_string(),
            gate_addr: ui.get_gate_addr().to_string(),
            dispatch_ip: ui.get_dispatch_ip().to_string(),
            dispatch_port: ui.get_dispatch_port().to_string(),
            mitm_ca_cert_path: ui.get_mitm_ca_cert_path().to_string(),
            mitm_ca_key_path: ui.get_mitm_ca_key_path().to_string(),
            tls_cert_path: ui.get_tls_cert_path().to_string(),
            tls_key_path: ui.get_tls_key_path().to_string(),
            dispatch_version_azhash: ui.get_dispatch_version_azhash().to_string(),
            dispatch_version_cvhash: ui.get_dispatch_version_cvhash().to_string(),
            dispatch_version_l2dhash: ui.get_dispatch_version_l2dhash().to_string(),
            dispatch_version_pichash: ui.get_dispatch_version_pichash().to_string(),
            dispatch_version_bgmhash: ui.get_dispatch_version_bgmhash().to_string(),
            dispatch_version_paintinghash: ui.get_dispatch_version_paintinghash().to_string(),
            dispatch_version_mangahash: ui.get_dispatch_version_mangahash().to_string(),
            dispatch_version_cipherhash: ui.get_dispatch_version_cipherhash().to_string(),
            dispatch_version_dormhash: ui.get_dispatch_version_dormhash().to_string(),
            dispatch_server_ip: ui.get_dispatch_server_ip().to_string(),
            dispatch_server_port: ui.get_dispatch_server_port().to_string(),
            dispatch_server_name: ui.get_dispatch_server_name().to_string(),
        }
    }

    fn apply_to_ui(self, ui: &AppWindow) {
        ui.set_database_url(self.database_url.into());
        ui.set_sdk_http_addr(self.sdk_http_addr.into());
        ui.set_sdk_https_addr(self.sdk_https_addr.into());
        ui.set_sdk_ip(self.sdk_ip.into());
        ui.set_sdk_proxy_addr(self.sdk_proxy_addr.into());
        ui.set_sdk_proxy_upstream_addr(self.sdk_proxy_upstream_addr.into());
        ui.set_dispatch_addr(self.dispatch_addr.into());
        ui.set_gate_addr(self.gate_addr.into());
        ui.set_dispatch_ip(self.dispatch_ip.into());
        ui.set_dispatch_port(self.dispatch_port.into());
        ui.set_mitm_ca_cert_path(self.mitm_ca_cert_path.into());
        ui.set_mitm_ca_key_path(self.mitm_ca_key_path.into());
        ui.set_tls_cert_path(self.tls_cert_path.into());
        ui.set_tls_key_path(self.tls_key_path.into());
        ui.set_dispatch_version_azhash(self.dispatch_version_azhash.into());
        ui.set_dispatch_version_cvhash(self.dispatch_version_cvhash.into());
        ui.set_dispatch_version_l2dhash(self.dispatch_version_l2dhash.into());
        ui.set_dispatch_version_pichash(self.dispatch_version_pichash.into());
        ui.set_dispatch_version_bgmhash(self.dispatch_version_bgmhash.into());
        ui.set_dispatch_version_paintinghash(self.dispatch_version_paintinghash.into());
        ui.set_dispatch_version_mangahash(self.dispatch_version_mangahash.into());
        ui.set_dispatch_version_cipherhash(self.dispatch_version_cipherhash.into());
        ui.set_dispatch_version_dormhash(self.dispatch_version_dormhash.into());
        ui.set_dispatch_server_ip(self.dispatch_server_ip.into());
        ui.set_dispatch_server_port(self.dispatch_server_port.into());
        ui.set_dispatch_server_name(self.dispatch_server_name.into());
    }

    fn into_config(self) -> Result<Config> {
        let database_url = required(self.database_url, "Database URL")?;
        if !database_url.starts_with("sqlite:") {
            bail!("Database URL must be a SQLite URL such as sqlite://cheshire.sqlite");
        }

        let sdk_ip = advertised_ipv4(self.sdk_ip, "SDK advertised IPv4")?;
        let dispatch_ip = advertised_ipv4(self.dispatch_ip, "Dispatch advertised IPv4")?;
        let dispatch_port = parse_port(&self.dispatch_port, "Dispatch advertised port")?;
        let server_ip = advertised_ipv4(self.dispatch_server_ip, "Dispatch server IPv4")?;
        let server_port = parse_port(&self.dispatch_server_port, "Dispatch server port")?;
        let server_name = required(self.dispatch_server_name, "Dispatch server name")?;
        let dispatch_version = vec![
            required(self.dispatch_version_azhash, "azhash version")?,
            required(self.dispatch_version_cvhash, "cvhash version")?,
            required(self.dispatch_version_l2dhash, "l2dhash version")?,
            required(self.dispatch_version_pichash, "pichash version")?,
            required(self.dispatch_version_bgmhash, "bgmhash version")?,
            required(self.dispatch_version_paintinghash, "paintinghash version")?,
            required(self.dispatch_version_mangahash, "mangahash version")?,
            required(self.dispatch_version_cipherhash, "cipherhash version")?,
            required(self.dispatch_version_dormhash, "dormhash version")?,
            "dTag-1".to_string(),
        ];
        let dispatch_server = Serverinfo {
            ids: vec![1],
            ip: server_ip.clone(),
            port: server_port as u32,
            state: 0,
            name: server_name,
            tag_state: Some(2),
            sort: Some(0),
            proxy_ip: Some(server_ip),
            proxy_port: Some(server_port as u32),
        };

        Ok(Config {
            database_url,
            sdk_http_addr: parse_socket(&self.sdk_http_addr, "SDK HTTP bind address")?,
            sdk_https_addr: parse_socket(&self.sdk_https_addr, "SDK HTTPS bind address")?,
            sdk_ip,
            sdk_proxy_addr: parse_socket(&self.sdk_proxy_addr, "SDK proxy bind address")?,
            sdk_proxy_upstream_addr: parse_socket(
                &self.sdk_proxy_upstream_addr,
                "SDK proxy upstream",
            )?,
            dispatch_addr: parse_socket(&self.dispatch_addr, "Dispatch bind address")?,
            gate_addr: parse_socket(&self.gate_addr, "Gate bind address")?,
            dispatch_ip,
            dispatch_port,
            dispatch_version,
            dispatch_servers: vec![dispatch_server],
            mitm_ca_cert_path: required_path(self.mitm_ca_cert_path, "MITM CA certificate path")?,
            mitm_ca_key_path: required_path(self.mitm_ca_key_path, "MITM CA key path")?,
            tls_cert_path: required_path(self.tls_cert_path, "SDK TLS certificate path")?,
            tls_key_path: required_path(self.tls_key_path, "SDK TLS key path")?,
        })
    }
}

pub fn populate(ui: &AppWindow, config: &Config) -> Result<()> {
    FormValues::from_config(config)?.apply_to_ui(ui);
    Ok(())
}

pub fn collect(ui: &AppWindow) -> Result<Config> {
    FormValues::from_ui(ui).into_config()
}

fn required(value: String, label: &str) -> Result<String> {
    let value = value.trim().to_string();
    if value.is_empty() {
        bail!("{label} cannot be empty");
    }
    Ok(value)
}

fn required_path(value: String, label: &str) -> Result<PathBuf> {
    Ok(PathBuf::from(required(value, label)?))
}

fn parse_socket(value: &str, label: &str) -> Result<SocketAddr> {
    let address = value
        .trim()
        .parse::<SocketAddr>()
        .with_context(|| format!("{label} must use IP:port format"))?;
    if address.port() == 0 {
        bail!("{label} cannot use port 0 in the desktop application");
    }
    Ok(address)
}

fn parse_port(value: &str, label: &str) -> Result<u16> {
    let port = value
        .trim()
        .parse::<u16>()
        .with_context(|| format!("{label} must be a number from 1 to 65535"))?;
    if port == 0 {
        bail!("{label} must be between 1 and 65535");
    }
    Ok(port)
}

fn advertised_ipv4(value: String, label: &str) -> Result<String> {
    let value = required(value, label)?;
    let address = value
        .parse::<Ipv4Addr>()
        .with_context(|| format!("{label} must be an IPv4 address"))?;
    if address.is_unspecified() {
        bail!("{label} cannot be 0.0.0.0 because clients cannot connect to a wildcard address");
    }
    if address.is_broadcast() {
        bail!("{label} cannot be the broadcast address 255.255.255.255");
    }
    Ok(address.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_config_round_trips_through_form_values() {
        let original = Config::default();
        let rebuilt = FormValues::from_config(&original)
            .unwrap()
            .into_config()
            .unwrap();

        assert_eq!(rebuilt.database_url, original.database_url);
        assert_eq!(rebuilt.sdk_http_addr, original.sdk_http_addr);
        assert_eq!(rebuilt.dispatch_ip, original.dispatch_ip);
        assert_eq!(rebuilt.dispatch_version, original.dispatch_version);
        assert_eq!(rebuilt.dispatch_servers, original.dispatch_servers);
    }

    #[test]
    fn automatic_dispatch_values_are_rebuilt() {
        let mut form = FormValues::from_config(&Config::default()).unwrap();
        form.dispatch_server_ip = "192.168.1.20".to_string();
        form.dispatch_server_port = "23456".to_string();
        form.dispatch_server_name = "Test Server".to_string();

        let rebuilt = form.into_config().unwrap();
        let server = &rebuilt.dispatch_servers[0];

        assert_eq!(rebuilt.dispatch_version.len(), 10);
        assert_eq!(rebuilt.dispatch_version.last().unwrap(), "dTag-1");
        assert_eq!(server.ids, vec![1]);
        assert_eq!(server.ip, "192.168.1.20");
        assert_eq!(server.port, 23456);
        assert_eq!(server.state, 0);
        assert_eq!(server.name, "Test Server");
        assert_eq!(server.tag_state, Some(2));
        assert_eq!(server.sort, Some(0));
        assert_eq!(server.proxy_ip.as_deref(), Some("192.168.1.20"));
        assert_eq!(server.proxy_port, Some(23456));
    }

    #[test]
    fn missing_dispatch_version_names_the_entry() {
        let mut form = FormValues::from_config(&Config::default()).unwrap();
        form.dispatch_version_paintinghash.clear();

        let error = form.into_config().err().unwrap().to_string();

        assert!(error.contains("paintinghash version"));
    }

    #[test]
    fn wildcard_advertised_ip_is_rejected() {
        let mut form = FormValues::from_config(&Config::default()).unwrap();
        form.sdk_ip = "0.0.0.0".to_string();

        let error = form.into_config().err().unwrap().to_string();

        assert!(error.contains("clients cannot connect"));
    }

    #[test]
    fn invalid_bind_address_names_the_field() {
        let mut form = FormValues::from_config(&Config::default()).unwrap();
        form.gate_addr = "localhost:21280".to_string();

        let error = form.into_config().err().unwrap().to_string();

        assert!(error.contains("Gate bind address"));
    }
}
