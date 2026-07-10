//! OpenTimestamps calendar submission — the external anchor that lets a
//! batch's Merkle root be verified without trusting this HSIP instance's
//! own database. See `anchor_job.rs` for the batching/DB orchestration that
//! calls into this module; this file is the network client only.
//!
//! ## Scope of this MVP
//!
//! This submits a digest to public OpenTimestamps calendars and stores
//! whatever they hand back as an opaque blob (`decision_anchors.ots_proof`)
//! — it does not parse or validate the OpenTimestamps `.ots` binary format,
//! and it does not yet poll calendars to "upgrade" a pending commitment
//! into one confirmed by a mined Bitcoin block. Both are natural next steps
//! once submission is confirmed working against real calendar servers.
//!
//! **This client could not be live-tested against real calendars during
//! development**: the sandbox this was built in blocks outbound HTTPS to
//! arbitrary hosts by policy (confirmed via `alice.btc.calendar
//! .opentimestamps.org` and `bob.btc.calendar.opentimestamps.org` both
//! getting a `403` on the CONNECT tunnel, logged by the sandbox's own
//! egress proxy). Verify connectivity in a real deployment before relying
//! on this for anything.

use anyhow::{bail, Context, Result};

/// Public OpenTimestamps calendar servers. Submission tries each in order
/// and returns the first success — one calendar's commitment is enough for
/// a meaningful anchor; trying more than one just improves availability.
pub const DEFAULT_CALENDARS: &[&str] = &[
    "https://alice.btc.calendar.opentimestamps.org",
    "https://bob.btc.calendar.opentimestamps.org",
    "https://finney.calendar.eternitywall.com",
];

/// One calendar's response to a digest submission: its raw, opaque,
/// not-yet-Bitcoin-confirmed commitment bytes.
#[derive(Debug, Clone)]
pub struct CalendarReceipt {
    pub calendar_url: String,
    pub response_bytes: Vec<u8>,
}

/// Submit a 32-byte digest (a batch's Merkle root) to the given calendar
/// servers per the OpenTimestamps calendar HTTP protocol: `POST
/// <calendar>/digest` with the raw digest bytes as the body. Tries each
/// calendar in turn; returns the first success.
pub async fn submit_digest_to(calendars: &[&str], digest: &[u8; 32]) -> Result<CalendarReceipt> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(concat!("hsip-api/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("failed to build HTTP client")?;

    let mut last_err: Option<String> = None;
    for calendar in calendars {
        let url = format!("{calendar}/digest");
        match client
            .post(&url)
            .header("Content-Type", "application/x-www-form-urlencoded")
            .body(digest.to_vec())
            .send()
            .await
        {
            Ok(resp) if resp.status().is_success() => match resp.bytes().await {
                Ok(body) => {
                    return Ok(CalendarReceipt {
                        calendar_url: (*calendar).to_string(),
                        response_bytes: body.to_vec(),
                    });
                }
                Err(e) => last_err = Some(format!("{calendar}: failed to read body: {e}")),
            },
            Ok(resp) => last_err = Some(format!("{calendar}: HTTP {}", resp.status())),
            Err(e) => last_err = Some(format!("{calendar}: request failed: {e}")),
        }
    }

    bail!(
        "all OpenTimestamps calendars unreachable or failed: {}",
        last_err.unwrap_or_default()
    );
}

#[cfg(test)]
mod tests {
    use super::*;
    use wiremock::matchers::{method, path};
    use wiremock::{Mock, MockServer, ResponseTemplate};

    #[tokio::test]
    async fn submits_raw_digest_bytes_and_returns_response_body() {
        let mock = MockServer::start().await;
        let canned_response = b"\x00fake-pending-ots-commitment".to_vec();

        Mock::given(method("POST"))
            .and(path("/digest"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(canned_response.clone()))
            .mount(&mock)
            .await;

        let digest = [0x42u8; 32];
        let receipt = submit_digest_to(&[&mock.uri()], &digest).await.unwrap();

        assert_eq!(receipt.response_bytes, canned_response);
        assert_eq!(receipt.calendar_url, mock.uri());
    }

    #[tokio::test]
    async fn request_body_is_exactly_the_raw_digest() {
        let mock = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/digest"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"ok".to_vec()))
            .mount(&mock)
            .await;

        let digest = [0x11u8; 32];
        let _ = submit_digest_to(&[&mock.uri()], &digest).await.unwrap();

        let requests = mock.received_requests().await.unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(requests[0].body, digest.to_vec());
    }

    #[tokio::test]
    async fn falls_over_to_next_calendar_when_first_fails() {
        let dead = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/digest"))
            .respond_with(ResponseTemplate::new(503))
            .mount(&dead)
            .await;

        let alive = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/digest"))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(b"second calendar".to_vec()))
            .mount(&alive)
            .await;

        let digest = [0x99u8; 32];
        let receipt = submit_digest_to(&[&dead.uri(), &alive.uri()], &digest)
            .await
            .unwrap();

        assert_eq!(receipt.calendar_url, alive.uri());
        assert_eq!(receipt.response_bytes, b"second calendar".to_vec());
    }

    #[tokio::test]
    async fn all_calendars_failing_is_an_error_not_a_panic() {
        let dead = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/digest"))
            .respond_with(ResponseTemplate::new(500))
            .mount(&dead)
            .await;

        let digest = [0x01u8; 32];
        let result = submit_digest_to(&[&dead.uri()], &digest).await;
        assert!(result.is_err());
    }
}
