//! Optional mutual TLS for the HTTPS server (`[server.tls] client_ca_path`).
//!
//! HSIP has no dedicated node-to-node network protocol today — "federated
//! trust" (`routes::trust`) is offline key registration + local signature
//! verification, not a live channel between HSIP instances. THREAT_MODEL.md
//! nonetheless flagged a real, generic gap: HSIP's own HTTPS server (the
//! `[server.tls]` config that has existed since the start) only ever
//! authenticated itself to clients, never the reverse — any TLS client that
//! knew a bearer token could connect, with nothing enforced at the
//! transport layer. This module closes that for anyone who wants it: when
//! `client_ca_path` names a CA certificate file, the server refuses to
//! complete a TLS handshake with any client that doesn't present a
//! certificate signed by that CA — on top of, not instead of, the existing
//! bearer-token auth every request still goes through. This is what an
//! operator running multiple HSIP nodes (or a partner/regulator's system)
//! that connect to each other's APIs over HTTPS would configure identically
//! on both ends to authenticate each other at the transport layer, which is
//! the literal gap this closes — it just isn't specific to HSIP-to-HSIP
//! traffic, since no such specific protocol exists to restrict it to.
//!
//! Fully backward compatible: `client_ca_path: None` (the default, and the
//! only option before this) takes the exact same code path
//! (`RustlsConfig::from_pem_file`) as before this module existed.

use anyhow::{Context, Result};
use axum_server::tls_rustls::RustlsConfig;
use rustls::pki_types::pem::PemObject;
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::server::WebPkiClientVerifier;
use rustls::{RootCertStore, ServerConfig};
use std::sync::Arc;

/// Builds the TLS config the server actually binds with. `client_ca_path`
/// absent → today's server-only-TLS behavior, unchanged. `client_ca_path`
/// present → mutual TLS: every client must present a certificate signed by
/// a CA in that file.
pub async fn build_rustls_config(
    cert_path: &str,
    key_path: &str,
    client_ca_path: Option<&str>,
) -> Result<RustlsConfig> {
    let Some(ca_path) = client_ca_path else {
        return RustlsConfig::from_pem_file(cert_path, key_path)
            .await
            .context("failed to load TLS certificate/key");
    };

    let cert_path = cert_path.to_string();
    let key_path = key_path.to_string();
    let ca_path = ca_path.to_string();

    // Certificate/key/CA parsing is synchronous (rustls) — run on a
    // blocking thread the same way axum-server's own `from_pem_file` does,
    // so it doesn't block the async runtime on file I/O.
    let server_config =
        tokio::task::spawn_blocking(move || build_server_config(&cert_path, &key_path, &ca_path))
            .await
            .context("mTLS config task panicked")??;

    Ok(RustlsConfig::from_config(Arc::new(server_config)))
}

fn build_server_config(cert_path: &str, key_path: &str, ca_path: &str) -> Result<ServerConfig> {
    let cert_chain = load_certs(cert_path)
        .with_context(|| format!("failed to load TLS certificate: {cert_path}"))?;
    let private_key = load_private_key(key_path)
        .with_context(|| format!("failed to load TLS private key: {key_path}"))?;
    let client_verifier = load_client_verifier(ca_path)
        .with_context(|| format!("failed to load client CA certificate(s): {ca_path}"))?;

    let mut config = ServerConfig::builder()
        .with_client_cert_verifier(client_verifier)
        .with_single_cert(cert_chain, private_key)
        .context("invalid TLS certificate/key pair")?;
    // Same ALPN protocols axum-server's own from_pem/from_pem_file set —
    // without this, HTTP/2 negotiation over this hand-built config would
    // silently not offer h2.
    config.alpn_protocols = vec![b"h2".to_vec(), b"http/1.1".to_vec()];

    Ok(config)
}

fn load_certs(path: &str) -> Result<Vec<CertificateDer<'static>>> {
    let bytes = std::fs::read(path)?;
    CertificateDer::pem_slice_iter(&bytes)
        .collect::<Result<Vec<_>, _>>()
        .context("failed to parse PEM certificate(s)")
}

fn load_private_key(path: &str) -> Result<PrivateKeyDer<'static>> {
    let bytes = std::fs::read(path)?;
    // A PEM file may contain other sections before the key (e.g. a cert) —
    // scan the whole file for the first parseable private key, same as
    // axum-server's own `config_from_pem`.
    for item in PrivateKeyDer::pem_slice_iter(&bytes) {
        if let Ok(key) = item {
            return Ok(key);
        }
    }
    anyhow::bail!("no private key found in {path}")
}

/// Loads one or more CA certificates from `ca_path` and builds a client
/// certificate verifier that requires (not merely allows) every connecting
/// client to present a certificate chaining to one of them. Kept as its
/// own function — separate from TLS-listener setup — so it can be unit
/// tested without binding a real socket.
fn load_client_verifier(
    ca_path: &str,
) -> Result<Arc<dyn rustls::server::danger::ClientCertVerifier>> {
    let bytes = std::fs::read(ca_path)?;
    let mut roots = RootCertStore::empty();
    let mut loaded = 0usize;
    for cert in CertificateDer::pem_slice_iter(&bytes) {
        roots
            .add(cert.context("failed to parse a certificate in client_ca_path")?)
            .context("failed to add CA certificate to trust store")?;
        loaded += 1;
    }
    if loaded == 0 {
        anyhow::bail!("no certificates found in {ca_path}");
    }

    WebPkiClientVerifier::builder(Arc::new(roots))
        .build()
        .context("failed to build client certificate verifier")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    /// Generates a throwaway CA + leaf certificate pair via the system
    /// `openssl` CLI into `dir`, returning (ca_pem_path, cert_pem_path,
    /// key_pem_path). Real X.509 certificates, not mocked bytes — this is
    /// exercising the actual PEM/cert parsing path, not a stand-in for it.
    fn generate_test_ca(dir: &std::path::Path) -> std::path::PathBuf {
        let ca_key = dir.join("ca-key.pem");
        let ca_cert = dir.join("ca-cert.pem");
        let status = Command::new("openssl")
            .args([
                "req",
                "-x509",
                "-newkey",
                "rsa:2048",
                "-nodes",
                "-keyout",
                ca_key.to_str().unwrap(),
                "-out",
                ca_cert.to_str().unwrap(),
                "-days",
                "1",
                "-subj",
                "/CN=HSIP Test CA",
            ])
            .status()
            .expect("openssl must be available to run this test");
        assert!(status.success(), "openssl CA generation failed");
        ca_cert
    }

    /// `ServerConfig::builder()`/`WebPkiClientVerifier::builder()` need a
    /// process-wide default `CryptoProvider` installed before first use —
    /// see the identical guard in `main.rs::main()`. Tests run in their own
    /// binary that never executes `main()`, so each test installs it too;
    /// `install_default()` is safe to call more than once (later calls just
    /// return `Err`, which is ignored).
    fn ensure_crypto_provider() {
        let _ = rustls::crypto::aws_lc_rs::default_provider().install_default();
    }

    #[test]
    fn load_client_verifier_accepts_a_real_ca_certificate() {
        ensure_crypto_provider();
        let dir = std::env::temp_dir().join(format!("hsip-mtls-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let ca_cert = generate_test_ca(&dir);

        let result = load_client_verifier(ca_cert.to_str().unwrap());
        assert!(
            result.is_ok(),
            "a real, valid CA certificate must build a working client verifier: {:?}",
            result.err()
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_client_verifier_rejects_garbage_input() {
        let dir = std::env::temp_dir().join(format!("hsip-mtls-test-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        let not_a_cert = dir.join("not-a-cert.pem");
        std::fs::write(&not_a_cert, b"this is not a PEM certificate\n").unwrap();

        let result = load_client_verifier(not_a_cert.to_str().unwrap());
        assert!(
            result.is_err(),
            "garbage input must not silently produce a (vacuously trust-nobody) verifier"
        );

        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn load_client_verifier_rejects_missing_file() {
        let result = load_client_verifier("/nonexistent/path/ca.pem");
        assert!(result.is_err());
    }
}
