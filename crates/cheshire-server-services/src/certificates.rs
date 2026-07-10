use std::fs;
use std::path::Path;
use std::time::{Duration, SystemTime};

use anyhow::{Context, Result};
use cheshire_server_core::Config;
use hudsucker::rcgen::{
    BasicConstraints, CertificateParams, DistinguishedName, DnType, DnValue,
    ExtendedKeyUsagePurpose, IsCa, Issuer, KeyPair, KeyUsagePurpose,
};

const ONE_DAY: Duration = Duration::from_secs(24 * 60 * 60);
const CA_VALIDITY: Duration = Duration::from_secs(20 * 365 * 24 * 60 * 60);
const TLS_VALIDITY: Duration = Duration::from_secs(10 * 365 * 24 * 60 * 60);

pub fn ensure(config: &Config) -> Result<()> {
    let ca_generated = ensure_ca(&config.mitm_ca_cert_path, &config.mitm_ca_key_path)?;
    ensure_tls(
        &config.tls_cert_path,
        &config.tls_key_path,
        &config.mitm_ca_cert_path,
        &config.mitm_ca_key_path,
        ca_generated,
    )?;
    Ok(())
}

fn ensure_ca(cert_path: &Path, key_path: &Path) -> Result<bool> {
    if cert_path.is_file() && key_path.is_file() {
        return Ok(false);
    }

    tracing::info!(
        cert = %cert_path.display(),
        key = %key_path.display(),
        "generating persistent MITM CA"
    );

    let key = KeyPair::generate().context("generate MITM CA private key")?;
    let mut params = CertificateParams::new(Vec::<String>::new())?;
    set_validity(&mut params, CA_VALIDITY);
    params.distinguished_name = distinguished_name("CheshireLane Local CA");
    params.is_ca = IsCa::Ca(BasicConstraints::Unconstrained);
    params.key_usages = vec![
        KeyUsagePurpose::DigitalSignature,
        KeyUsagePurpose::KeyCertSign,
        KeyUsagePurpose::CrlSign,
    ];

    let cert = params
        .self_signed(&key)
        .context("sign MITM CA certificate")?;
    write_public(cert_path, cert.pem().as_bytes())?;
    write_private(key_path, key.serialize_pem().as_bytes())?;
    Ok(true)
}

fn ensure_tls(
    cert_path: &Path,
    key_path: &Path,
    ca_cert_path: &Path,
    ca_key_path: &Path,
    force: bool,
) -> Result<()> {
    if !force && cert_path.is_file() && key_path.is_file() {
        return Ok(());
    }

    tracing::info!(
        cert = %cert_path.display(),
        key = %key_path.display(),
        "generating SDK TLS certificate"
    );

    let ca_cert_pem = fs::read_to_string(ca_cert_path)
        .with_context(|| format!("read MITM CA certificate {}", ca_cert_path.display()))?;
    let ca_key_pem = fs::read_to_string(ca_key_path)
        .with_context(|| format!("read MITM CA key {}", ca_key_path.display()))?;
    let ca_key = KeyPair::from_pem(&ca_key_pem).context("parse MITM CA private key")?;
    let issuer =
        Issuer::from_ca_cert_pem(&ca_cert_pem, ca_key).context("parse MITM CA certificate")?;

    let key = KeyPair::generate().context("generate SDK TLS private key")?;
    let mut params = CertificateParams::new(vec![
        "localhost".to_string(),
        "127.0.0.1".to_string(),
        "::1".to_string(),
        "jp-sdk-api.yostarplat.com".to_string(),
        "en-sdk-api.yostarplat.com".to_string(),
    ])?;
    set_validity(&mut params, TLS_VALIDITY);
    params.distinguished_name = distinguished_name("Akiko97");
    params.key_usages = vec![KeyUsagePurpose::DigitalSignature];
    params.extended_key_usages = vec![ExtendedKeyUsagePurpose::ServerAuth];
    params.use_authority_key_identifier_extension = true;

    let cert = params
        .signed_by(&key, &issuer)
        .context("sign SDK TLS certificate")?;
    write_public(cert_path, cert.pem().as_bytes())?;
    write_private(key_path, key.serialize_pem().as_bytes())?;
    Ok(())
}

fn set_validity(params: &mut CertificateParams, validity: Duration) {
    let now = SystemTime::now();
    params.not_before = now.checked_sub(ONE_DAY).unwrap_or(now).into();
    params.not_after = now.checked_add(validity).unwrap_or(now).into();
}

fn distinguished_name(common_name: &str) -> DistinguishedName {
    let mut name = DistinguishedName::new();
    name.push(
        DnType::CountryName,
        DnValue::PrintableString("CN".try_into().expect("static country name is valid")),
    );
    name.push(DnType::StateOrProvinceName, "Beijing");
    name.push(DnType::LocalityName, "Beijing");
    name.push(DnType::OrganizationName, "Dev.Akiko97");
    name.push(DnType::OrganizationalUnitName, "dev");
    name.push(DnType::CommonName, common_name);
    name.push(
        DnType::CustomDnType(vec![1, 2, 840, 113549, 1, 9, 1]),
        DnValue::Ia5String(
            "akiko97@akiko97.dev"
                .try_into()
                .expect("static email address is valid"),
        ),
    );
    name
}

fn write_public(path: &Path, data: &[u8]) -> Result<()> {
    create_parent(path)?;
    fs::write(path, data).with_context(|| format!("write certificate {}", path.display()))?;
    Ok(())
}

fn write_private(path: &Path, data: &[u8]) -> Result<()> {
    create_parent(path)?;
    fs::write(path, data).with_context(|| format!("write private key {}", path.display()))?;

    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        fs::set_permissions(path, fs::Permissions::from_mode(0o600))
            .with_context(|| format!("restrict private key permissions {}", path.display()))?;
    }

    Ok(())
}

fn create_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)
            .with_context(|| format!("create certificate directory {}", parent.display()))?;
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use cheshire_server_proto::p10::Serverinfo;

    use super::*;

    #[test]
    fn generates_persistent_ca_and_tls_pair() {
        let id = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let root = std::env::temp_dir().join(format!("cheshire-certificates-{id}"));
        let config = test_config(&root);

        ensure(&config).unwrap();
        let first_ca = fs::read(&config.mitm_ca_cert_path).unwrap();
        let first_ca_key = fs::read(&config.mitm_ca_key_path).unwrap();
        let first_tls = fs::read(&config.tls_cert_path).unwrap();
        let first_tls_key = fs::read(&config.tls_key_path).unwrap();

        let ca_key = KeyPair::from_pem(std::str::from_utf8(&first_ca_key).unwrap()).unwrap();
        Issuer::from_ca_cert_pem(std::str::from_utf8(&first_ca).unwrap(), ca_key).unwrap();

        ensure(&config).unwrap();
        assert_eq!(first_ca, fs::read(&config.mitm_ca_cert_path).unwrap());
        assert_eq!(first_ca_key, fs::read(&config.mitm_ca_key_path).unwrap());
        assert_eq!(first_tls, fs::read(&config.tls_cert_path).unwrap());
        assert_eq!(first_tls_key, fs::read(&config.tls_key_path).unwrap());

        let _ = fs::remove_dir_all(root);
    }

    fn test_config(root: &Path) -> Config {
        Config {
            database_url: "sqlite::memory:".to_string(),
            assets_dir: root.join("assets"),
            sdk_http_addr: "127.0.0.1:0".parse().unwrap(),
            sdk_https_addr: "127.0.0.1:0".parse().unwrap(),
            sdk_ip: "127.0.0.1".to_string(),
            sdk_proxy_addr: "127.0.0.1:0".parse().unwrap(),
            sdk_proxy_upstream_addr: "127.0.0.1:0".parse().unwrap(),
            dispatch_addr: "127.0.0.1:0".parse().unwrap(),
            gate_addr: "127.0.0.1:0".parse().unwrap(),
            dispatch_ip: "127.0.0.1".to_string(),
            dispatch_port: 0,
            dispatch_version: vec![],
            dispatch_servers: Vec::<Serverinfo>::new(),
            mitm_ca_cert_path: root.join("assets/ca/ca-cert.cer"),
            mitm_ca_key_path: root.join("assets/ca/ca-key.pem"),
            tls_cert_path: root.join("assets/tls/cert.pem"),
            tls_key_path: root.join("assets/tls/key.pem"),
        }
    }
}
