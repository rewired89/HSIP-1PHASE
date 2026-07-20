//! OpenTimestamps calendar submission — the external anchor that lets a
//! batch's Merkle root be verified without trusting this HSIP instance's
//! own database. See `anchor_job.rs` for the batching/DB orchestration that
//! calls into this module; this file is the network client only.
//!
//! ## Scope of this MVP
//!
//! This submits a digest to public OpenTimestamps calendars and stores
//! whatever they hand back as an opaque blob (`decision_anchors.ots_proof`)
//! — it does not parse or fully verify the OpenTimestamps `.ots` binary
//! format's Merkle-path operations. It does now poll calendars to
//! "upgrade" a pending commitment once a mined Bitcoin block has confirmed
//! it — see `check_for_upgrade`/`contains_bitcoin_attestation` below and
//! `anchor_job::run_upgrade_cycle`.
//!
//! **Live-tested against real calendars, from a real unrestricted network**
//! (every sandbox this project had previously been developed in blocked
//! outbound HTTPS to arbitrary hosts by policy — this had only ever been
//! unit-tested against a mocked calendar until now). Ran a real `hsip-api`
//! server in desktop mode, recorded a decision, and let the anchor job
//! submit it for real: `GET /v1/decisions/:id/proof` came back
//! `ots_status: "pending"` with a genuine calendar receipt in `ots_proof`
//! — decoding those bytes shows `alice.btc.calendar.opentimestamps.org`'s
//! own URL embedded in its response, and the byte count matches a direct
//! curl/`Invoke-WebRequest` `POST <calendar>/digest` against the same
//! calendar. Not a placeholder or a mocked response — see THREAT_MODEL.md
//! §4.20 for the full verification writeup.

use anyhow::{bail, Context, Result};

/// The 8-byte tag OpenTimestamps uses to mark a `PendingAttestation` inside
/// a serialized proof — i.e. "a calendar has this digest queued but it
/// isn't in a mined Bitcoin block yet." Matches the reference implementation
/// (`opentimestamps/core/notary.py`'s `PendingAttestation.TAG`) and confirmed
/// empirically against this project's own real calendar response: this
/// exact byte sequence appeared immediately before the submitting calendar's
/// own URL in a genuine `alice.btc.calendar.opentimestamps.org` response —
/// see THREAT_MODEL.md §4.20.
const PENDING_ATTESTATION_TAG: [u8; 8] = [0x83, 0xdf, 0xe3, 0x0d, 0x2e, 0xf9, 0x0c, 0x8e];

/// The 8-byte tag OpenTimestamps uses to mark a `BitcoinBlockHeaderAttestation`
/// — i.e. "this digest's Merkle root has been confirmed inside a specific
/// mined Bitcoin block." Matches the reference implementation
/// (`opentimestamps/core/notary.py`'s `BitcoinBlockHeaderAttestation.TAG`).
pub const BITCOIN_ATTESTATION_TAG: [u8; 8] = [0x05, 0x88, 0x96, 0x0d, 0x73, 0xd7, 0x19, 0x01];

/// Whether a calendar's response contains a Bitcoin-block-header
/// attestation, i.e. this submission has been confirmed by a mined Bitcoin
/// block. This is a presence check on OpenTimestamps' own protocol-defined
/// byte tag, not a full parse of the `.ots` binary format or an independent
/// verification of the Merkle path against a real Bitcoin block header —
/// see this module's doc comment for that documented MVP scope. It trusts
/// the calendar's response the same way the initial "pending" submission
/// already does; it doesn't introduce a new, weaker trust assumption.
#[must_use]
pub fn contains_bitcoin_attestation(proof_bytes: &[u8]) -> bool {
    proof_bytes
        .windows(BITCOIN_ATTESTATION_TAG.len())
        .any(|w| w == BITCOIN_ATTESTATION_TAG)
}

/// Reads an OpenTimestamps-style base-128 varint (7 payload bits per byte,
/// high bit set = more bytes follow — same scheme as protobuf/LEB128)
/// starting at `*pos`, advancing `*pos` past it.
fn read_varuint(bytes: &[u8], pos: &mut usize) -> Option<u64> {
    let mut result: u64 = 0;
    let mut shift: u32 = 0;
    loop {
        let byte = *bytes.get(*pos)?;
        *pos += 1;
        result |= u64::from(byte & 0x7f) << shift;
        if byte & 0x80 == 0 {
            return Some(result);
        }
        shift += 7;
        if shift >= 64 {
            return None;
        }
    }
}

/// Extracts the calendar URI embedded in a stored `PendingAttestation`
/// proof — the same calendar that originally accepted this submission, and
/// the only one that has any record of it to check for an upgrade against.
/// `decision_anchors`/`audit_anchors` don't store the calendar URL in its
/// own column; it's already inside the `ots_proof` blob we stored at
/// submission time, so this reads it back out instead of adding a column.
///
/// Layout, confirmed against this project's own real calendar response
/// (THREAT_MODEL.md §4.20): `PENDING_ATTESTATION_TAG` (8 bytes), then an
/// outer body-length varint, then an inner URI-length varint, then the URI
/// as UTF-8 bytes.
#[must_use]
pub fn extract_pending_calendar_uri(proof_bytes: &[u8]) -> Option<String> {
    let tag_pos = proof_bytes
        .windows(PENDING_ATTESTATION_TAG.len())
        .position(|w| w == PENDING_ATTESTATION_TAG)?;
    let mut pos = tag_pos + PENDING_ATTESTATION_TAG.len();
    let _outer_len = read_varuint(proof_bytes, &mut pos)?;
    let inner_len = usize::try_from(read_varuint(proof_bytes, &mut pos)?).ok()?;
    let uri_bytes = proof_bytes.get(pos..pos.checked_add(inner_len)?)?;
    String::from_utf8(uri_bytes.to_vec()).ok()
}

/// Ask a calendar whether a previously-submitted digest has been upgraded
/// since — i.e. whether it's since been confirmed by a mined Bitcoin block —
/// per the OpenTimestamps calendar HTTP protocol: `GET
/// <calendar>/timestamp/<hex-digest>`. Returns `Ok(None)` for "the calendar
/// has nothing new for this digest right now" (any non-success response,
/// including the calendar simply not having upgraded it yet) — that's the
/// expected, common outcome on most checks, not an error. Returns `Err`
/// only when the calendar couldn't be reached at all.
pub async fn check_for_upgrade(calendar_url: &str, digest: &[u8; 32]) -> Result<Option<Vec<u8>>> {
    let client = reqwest::Client::builder()
        .timeout(std::time::Duration::from_secs(15))
        .user_agent(concat!("hsip-api/", env!("CARGO_PKG_VERSION")))
        .build()
        .context("failed to build HTTP client")?;

    let url = format!("{calendar_url}/timestamp/{}", hex::encode(digest));
    let resp = client
        .get(&url)
        .send()
        .await
        .context("upgrade check request failed")?;

    if !resp.status().is_success() {
        return Ok(None);
    }

    let body = resp
        .bytes()
        .await
        .context("failed to read upgrade check response body")?;
    Ok(Some(body.to_vec()))
}

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

    /// The exact 137 bytes `alice.btc.calendar.opentimestamps.org` returned
    /// for a real submission during THREAT_MODEL.md §4.20's live
    /// verification — a real fixture, not synthesized, so these tests prove
    /// `extract_pending_calendar_uri`/`contains_bitcoin_attestation` work
    /// against what a real calendar actually sends, not just an idealized
    /// encoding of the format.
    const REAL_PENDING_PROOF: [u8; 137] = [
        0xf0, 0x08, 0x59, 0x0d, 0x1f, 0x57, 0x4f, 0xe7, 0xde, 0x32, 0x08, 0xf0, 0x10, 0xce, 0xa4,
        0x75, 0x87, 0xfe, 0x3e, 0xa3, 0x55, 0x80, 0x4f, 0x3a, 0xa5, 0x9f, 0x3e, 0xb8, 0xc7, 0x08,
        0xf1, 0x20, 0xdc, 0xc0, 0xb4, 0x5c, 0x96, 0x1a, 0x28, 0xf3, 0x28, 0x14, 0x2d, 0xb4, 0xe9,
        0xa2, 0xb4, 0xd0, 0x0d, 0xec, 0xfa, 0xe2, 0xb7, 0x10, 0x7a, 0xe4, 0x4d, 0x7a, 0x03, 0xf5,
        0xc0, 0xf4, 0x28, 0xf8, 0x08, 0xf1, 0x04, 0x6a, 0x5e, 0x89, 0x27, 0xf0, 0x08, 0x5c, 0xe5,
        0xaa, 0xaf, 0x6e, 0x5d, 0x06, 0x79, 0x00, 0x83, 0xdf, 0xe3, 0x0d, 0x2e, 0xf9, 0x0c, 0x8e,
        0x2e, 0x2d, 0x68, 0x74, 0x74, 0x70, 0x73, 0x3a, 0x2f, 0x2f, 0x61, 0x6c, 0x69, 0x63, 0x65,
        0x2e, 0x62, 0x74, 0x63, 0x2e, 0x63, 0x61, 0x6c, 0x65, 0x6e, 0x64, 0x61, 0x72, 0x2e, 0x6f,
        0x70, 0x65, 0x6e, 0x74, 0x69, 0x6d, 0x65, 0x73, 0x74, 0x61, 0x6d, 0x70, 0x73, 0x2e, 0x6f,
        0x72, 0x67,
    ];

    #[test]
    fn extracts_calendar_uri_from_a_real_pending_proof() {
        assert_eq!(
            extract_pending_calendar_uri(&REAL_PENDING_PROOF).as_deref(),
            Some("https://alice.btc.calendar.opentimestamps.org")
        );
    }

    #[test]
    fn a_real_pending_proof_does_not_contain_a_bitcoin_attestation() {
        assert!(!contains_bitcoin_attestation(&REAL_PENDING_PROOF));
    }

    #[test]
    fn detects_a_bitcoin_attestation_tag_when_present() {
        let mut upgraded = REAL_PENDING_PROOF.to_vec();
        upgraded.extend_from_slice(&BITCOIN_ATTESTATION_TAG);
        upgraded.extend_from_slice(b"...block header and merkle path bytes would follow...");
        assert!(contains_bitcoin_attestation(&upgraded));
    }

    #[test]
    fn extract_pending_calendar_uri_returns_none_for_garbage() {
        assert_eq!(
            extract_pending_calendar_uri(b"not an ots proof at all"),
            None
        );
        assert_eq!(extract_pending_calendar_uri(&[]), None);
    }

    #[tokio::test]
    async fn check_for_upgrade_returns_bytes_when_calendar_has_them() {
        let mock = MockServer::start().await;
        let digest = [0x42u8; 32];
        let upgraded_bytes = b"fully-confirmed-proof-bytes".to_vec();

        Mock::given(method("GET"))
            .and(path(format!("/timestamp/{}", hex::encode(digest))))
            .respond_with(ResponseTemplate::new(200).set_body_bytes(upgraded_bytes.clone()))
            .mount(&mock)
            .await;

        let result = check_for_upgrade(&mock.uri(), &digest).await.unwrap();
        assert_eq!(result, Some(upgraded_bytes));
    }

    #[tokio::test]
    async fn check_for_upgrade_returns_none_when_calendar_has_nothing_new() {
        let mock = MockServer::start().await;
        let digest = [0x77u8; 32];

        Mock::given(method("GET"))
            .and(path(format!("/timestamp/{}", hex::encode(digest))))
            .respond_with(ResponseTemplate::new(404))
            .mount(&mock)
            .await;

        let result = check_for_upgrade(&mock.uri(), &digest).await.unwrap();
        assert_eq!(result, None);
    }

    #[tokio::test]
    async fn check_for_upgrade_errors_when_calendar_is_unreachable() {
        let digest = [0x55u8; 32];
        // Nothing is listening on this port — a real connection failure,
        // distinct from the "calendar reachable but nothing new" 404 case.
        let result = check_for_upgrade("http://127.0.0.1:1", &digest).await;
        assert!(result.is_err());
    }
}
