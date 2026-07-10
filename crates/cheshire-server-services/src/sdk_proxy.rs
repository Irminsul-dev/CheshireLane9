use std::fs;
use std::net::SocketAddr;

use anyhow::{Context, Result};
use axum::http::uri::{Authority, Scheme};
use axum::http::{Method, Request, Uri};
use cheshire_server_core::Config;
use hudsucker::certificate_authority::RcgenAuthority;
use hudsucker::rcgen::{Issuer, KeyPair};
use hudsucker::rustls::crypto::ring;
use hudsucker::{Body, HttpContext, HttpHandler, Proxy, RequestOrResponse};

const SDK_HOSTS: [&str; 2] = ["jp-sdk-api.yostarplat.com", "en-sdk-api.yostarplat.com"];

#[derive(Clone)]
struct SdkRedirectHandler {
    upstream: Authority,
}

impl SdkRedirectHandler {
    fn new(upstream: SocketAddr) -> Self {
        Self {
            upstream: upstream
                .to_string()
                .parse()
                .expect("socket address must be a valid URI authority"),
        }
    }

    fn rewrite_uri(&self, uri: &Uri) -> Option<Uri> {
        if !is_sdk_host(uri.host()?) {
            return None;
        }

        let mut parts = uri.clone().into_parts();
        parts.scheme = Some(Scheme::HTTP);
        parts.authority = Some(self.upstream.clone());
        Some(Uri::from_parts(parts).expect("proxy upstream must produce a valid URI"))
    }
}

impl HttpHandler for SdkRedirectHandler {
    async fn handle_request(
        &mut self,
        ctx: &HttpContext,
        mut req: Request<Body>,
    ) -> RequestOrResponse {
        if req.method() == Method::CONNECT {
            return req.into();
        }

        if let Some(uri) = self.rewrite_uri(req.uri()) {
            tracing::debug!(
                client = %ctx.client_addr,
                original = %req.uri(),
                upstream = %uri,
                "redirecting SDK request"
            );
            *req.uri_mut() = uri;
        }

        req.into()
    }

    async fn should_intercept(&mut self, _ctx: &HttpContext, req: &Request<Body>) -> bool {
        req.uri().host().is_some_and(is_sdk_host)
    }
}

pub async fn serve(config: Config) -> Result<()> {
    let ca_key_pem = fs::read_to_string(&config.mitm_ca_key_path)
        .with_context(|| format!("read MITM CA key {}", config.mitm_ca_key_path.display()))?;
    let ca_cert_pem = fs::read_to_string(&config.mitm_ca_cert_path).with_context(|| {
        format!(
            "read MITM CA certificate {}",
            config.mitm_ca_cert_path.display()
        )
    })?;
    let ca_key = KeyPair::from_pem(&ca_key_pem).context("parse MITM CA key")?;
    let issuer =
        Issuer::from_ca_cert_pem(&ca_cert_pem, ca_key).context("parse MITM CA certificate")?;
    let provider = ring::default_provider();
    let ca = RcgenAuthority::new(issuer, 64, provider.clone());

    let proxy = Proxy::builder()
        .with_addr(config.sdk_proxy_addr)
        .with_ca(ca)
        .with_rustls_connector(provider)
        .with_http_handler(SdkRedirectHandler::new(config.sdk_proxy_upstream_addr))
        .build()
        .context("build SDK proxy")?;

    tracing::info!(
        listen = %config.sdk_proxy_addr,
        upstream = %config.sdk_proxy_upstream_addr,
        "SDK proxy listening"
    );
    proxy.start().await.context("serve SDK proxy")?;
    Ok(())
}

fn is_sdk_host(host: &str) -> bool {
    SDK_HOSTS
        .iter()
        .any(|sdk_host| host.eq_ignore_ascii_case(sdk_host))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rewrites_sdk_uri_and_preserves_path_and_query() {
        let handler = SdkRedirectHandler::new("127.0.0.1:21080".parse().unwrap());
        let uri: Uri = "https://jp-sdk-api.yostarplat.com/common/config?lang=ja"
            .parse()
            .unwrap();

        assert_eq!(
            handler.rewrite_uri(&uri).unwrap(),
            "http://127.0.0.1:21080/common/config?lang=ja"
        );
    }

    #[test]
    fn leaves_other_hosts_untouched() {
        let handler = SdkRedirectHandler::new("127.0.0.1:21080".parse().unwrap());
        let uri: Uri = "https://example.com/common/config".parse().unwrap();
        assert_eq!(handler.rewrite_uri(&uri), None);
    }
}
