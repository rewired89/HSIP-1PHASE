//! HSIP DNS Resolver
//!
//! A lightweight UDP DNS server that runs locally (127.0.0.1:5300 by default).
//! It checks every queried hostname against a curated tracker blocklist and
//! returns NXDOMAIN for known tracking/advertising domains. All other queries
//! are transparently forwarded to Cloudflare (1.1.1.1:53).
//!
//! # OS setup (to make the whole system use it)
//! - **Linux / macOS**: Add `nameserver 127.0.0.1` to `/etc/resolv.conf`, or
//!   point your network adapter's DNS to 127.0.0.1 in System Settings.
//! - **Windows**: Change your adapter's "Preferred DNS" to 127.0.0.1 in
//!   Network & Internet Settings.
//!
//! Note: standard DNS port 53 requires root/administrator privileges.
//! Use iptables/pf rules to redirect port 53 → 5300 for rootless operation,
//! or launch the binary with elevated privileges and set port to 53.

use std::collections::HashSet;
use std::collections::VecDeque;
use std::net::SocketAddr;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{SystemTime, UNIX_EPOCH};
use tokio::net::UdpSocket;
use tokio::sync::{broadcast, RwLock};

// ── Tracker blocklist ─────────────────────────────────────────────────────────
//
// Only domains that are *exclusively* used for tracking / advertising.
// We intentionally exclude dual-purpose domains (e.g. facebook.com, youtube.com)
// to avoid breaking normal browsing.

static TRACKER_DOMAINS: &[(&str, &str, &str)] = &[
    // (domain_suffix, vendor, category)
    // --- Google ---
    ("google-analytics.com", "Google Analytics", "Analytics"),
    ("googletagmanager.com", "Google Tag Manager", "Analytics"),
    ("doubleclick.net", "Google Ads", "Advertising"),
    ("googlesyndication.com", "Google AdSense", "Advertising"),
    // --- Meta ---
    ("connect.facebook.net", "Facebook Pixel", "Advertising"),
    ("graph.facebook.com", "Facebook App Events", "Advertising"),
    // --- Session Recording ---
    ("hotjar.com", "Hotjar", "Session Recording"),
    ("fullstory.com", "FullStory", "Session Recording"),
    ("clarity.ms", "Microsoft Clarity", "Session Recording"),
    ("logrocket.com", "LogRocket", "Session Recording"),
    ("mouseflow.com", "Mouseflow", "Session Recording"),
    ("crazyegg.com", "Crazy Egg", "Session Recording"),
    // --- Analytics ---
    ("mixpanel.com", "Mixpanel", "Analytics"),
    ("amplitude.com", "Amplitude", "Analytics"),
    ("api.segment.io", "Segment", "Analytics"),
    ("cdn.segment.com", "Segment", "Analytics"),
    ("heap.io", "Heap Analytics", "Analytics"),
    ("tealiumiq.com", "Tealium", "Analytics"),
    ("optimizely.com", "Optimizely", "A/B Testing"),
    // --- Ad Networks ---
    ("criteo.com", "Criteo", "Advertising"),
    ("taboola.com", "Taboola", "Advertising"),
    ("outbrain.com", "Outbrain", "Advertising"),
    ("adsrvr.org", "The Trade Desk", "Advertising"),
    ("adnxs.com", "Xandr/AppNexus", "Advertising"),
    ("rubiconproject.com", "Magnite/Rubicon", "Advertising"),
    ("pubmatic.com", "PubMatic", "Advertising"),
    ("scorecardresearch.com", "Scorecard Research", "Advertising"),
    ("quantserve.com", "QuantCast", "Advertising"),
    // --- Microsoft telemetry ---
    (
        "vortex.data.microsoft.com",
        "Windows Telemetry",
        "Telemetry",
    ),
    ("bat.bing.com", "Microsoft Ads", "Advertising"),
    ("applicationinsights.io", "Azure App Insights", "Analytics"),
    // --- Apple ---
    ("xp.apple.com", "Apple Analytics", "Analytics"),
    ("iadsdk.apple.com", "Apple Search Ads", "Advertising"),
];

// ── Helpers ───────────────────────────────────────────────────────────────────

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

/// Returns the vendor name if `hostname` matches any entry in the blocklist.
fn lookup_block(hostname: &str) -> Option<(&'static str, &'static str)> {
    let h = hostname.trim_end_matches('.').to_ascii_lowercase();
    for &(suffix, vendor, category) in TRACKER_DOMAINS {
        if h == suffix || h.ends_with(&format!(".{}", suffix)) {
            return Some((vendor, category));
        }
    }
    None
}

/// Parse the QNAME out of a raw DNS packet starting at `offset`.
/// Returns `(hostname, end_offset_past_QTYPE_QCLASS)` or `None` if malformed.
fn parse_qname(buf: &[u8], mut pos: usize) -> Option<(String, usize)> {
    let mut labels: Vec<String> = Vec::new();
    let mut final_pos: Option<usize> = None;
    let mut visited: HashSet<usize> = HashSet::new();

    loop {
        if pos >= buf.len() {
            return None;
        }
        if !visited.insert(pos) {
            return None;
        } // compression loop guard

        let b = buf[pos] as usize;

        if b & 0xC0 == 0xC0 {
            // Compression pointer — two bytes encoding a back-reference
            if pos + 1 >= buf.len() {
                return None;
            }
            if final_pos.is_none() {
                final_pos = Some(pos + 2);
            }
            pos = ((b & 0x3F) << 8) | buf[pos + 1] as usize;
            continue;
        }

        if b == 0 {
            // End of QNAME
            if final_pos.is_none() {
                final_pos = Some(pos + 1);
            }
            break;
        }

        let len = b;
        pos += 1;
        if pos + len > buf.len() {
            return None;
        }
        let label = std::str::from_utf8(&buf[pos..pos + len]).ok()?;
        labels.push(label.to_owned());
        pos += len;
    }

    let end = final_pos?;
    // Must have room for QTYPE (2) + QCLASS (2)
    if end + 4 > buf.len() {
        return None;
    }

    Some((labels.join("."), end + 4))
}

/// Build a minimal NXDOMAIN response from the original query bytes.
fn build_nxdomain(query: &[u8]) -> Vec<u8> {
    if query.len() < 12 {
        return vec![];
    }
    let mut resp = Vec::with_capacity(query.len());
    // Transaction ID — copy from query
    resp.push(query[0]);
    resp.push(query[1]);
    // Flags: QR=1, OPCODE=0, AA=0, TC=0, RD=copy, RA=1, RCODE=3 (NXDOMAIN)
    resp.push(0x80 | (query[2] & 0x01)); // preserve RD bit
    resp.push(0x83); // RA=1, RCODE=3
                     // QDCOUNT same as query
    resp.push(query[4]);
    resp.push(query[5]);
    // ANCOUNT = NSCOUNT = ARCOUNT = 0
    resp.extend_from_slice(&[0, 0, 0, 0, 0, 0]);
    // Copy the question section verbatim
    if query.len() > 12 {
        resp.extend_from_slice(&query[12..]);
    }
    resp
}

// ── Public types ──────────────────────────────────────────────────────────────

/// Live counters exposed via the API.
pub struct DnsStats {
    pub queries_total: AtomicU64,
    pub blocked_total: AtomicU64,
}

/// One entry in the recent-activity log.
#[derive(Clone, serde::Serialize)]
pub struct DnsLogEntry {
    pub domain: String,
    pub blocked: bool,
    pub vendor: Option<String>,
    pub category: Option<String>,
    pub timestamp_ms: i64,
}

/// Rolling circular buffer (max 200 entries) of recent DNS activity.
pub struct DnsLog {
    pub entries: RwLock<VecDeque<DnsLogEntry>>,
}

impl DnsLog {
    fn new() -> Self {
        Self {
            entries: RwLock::new(VecDeque::with_capacity(200)),
        }
    }

    async fn push(&self, entry: DnsLogEntry) {
        let mut q = self.entries.write().await;
        if q.len() >= 200 {
            q.pop_front();
        }
        q.push_back(entry);
    }
}

/// Handle returned by [`start`].  Clone it freely; call [`DnsHandle::shutdown`]
/// to stop the resolver.
#[derive(Clone)]
pub struct DnsHandle {
    pub stats: Arc<DnsStats>,
    pub log: Arc<DnsLog>,
    pub port: u16,
    shutdown_tx: broadcast::Sender<()>,
}

impl DnsHandle {
    /// Signal the resolver task to stop.
    pub fn shutdown(&self) {
        let _ = self.shutdown_tx.send(());
    }

    /// Total number of tracker domains in the blocklist.
    pub fn blocklist_size() -> usize {
        TRACKER_DOMAINS.len()
    }
}

// ── Entry point ───────────────────────────────────────────────────────────────

/// Start the DNS resolver on `127.0.0.1:<port>`.
///
/// Returns immediately with a [`DnsHandle`] — the resolver runs in a background
/// Tokio task.  Fails if the port is already in use or otherwise unbound.
pub async fn start(port: u16) -> std::io::Result<DnsHandle> {
    let addr: SocketAddr = format!("127.0.0.1:{}", port).parse().unwrap();
    let socket = Arc::new(UdpSocket::bind(addr).await?);

    tracing::info!("HSIP DNS resolver bound to {}", addr);

    let stats = Arc::new(DnsStats {
        queries_total: AtomicU64::new(0),
        blocked_total: AtomicU64::new(0),
    });
    let log = Arc::new(DnsLog::new());

    let (shutdown_tx, _) = broadcast::channel::<()>(4);

    let handle = DnsHandle {
        stats: Arc::clone(&stats),
        log: Arc::clone(&log),
        port,
        shutdown_tx: shutdown_tx.clone(),
    };

    tokio::spawn(resolver_loop(socket, stats, log, shutdown_tx.subscribe()));

    Ok(handle)
}

// ── Resolver loop ─────────────────────────────────────────────────────────────

async fn resolver_loop(
    socket: Arc<UdpSocket>,
    stats: Arc<DnsStats>,
    log: Arc<DnsLog>,
    mut stop: broadcast::Receiver<()>,
) {
    let upstream: SocketAddr = "1.1.1.1:53".parse().unwrap();
    let mut buf = [0u8; 512];

    loop {
        tokio::select! {
            _ = stop.recv() => {
                tracing::info!("HSIP DNS resolver shutting down");
                break;
            }
            result = socket.recv_from(&mut buf) => {
                let (n, client) = match result {
                    Ok(v)  => v,
                    Err(e) => {
                        tracing::warn!("DNS recv error: {}", e);
                        continue;
                    }
                };
                let query   = buf[..n].to_vec();
                let sock2   = Arc::clone(&socket);
                let stats2  = Arc::clone(&stats);
                let log2    = Arc::clone(&log);

                tokio::spawn(async move {
                    handle_query(sock2, stats2, log2, query, client, upstream).await;
                });
            }
        }
    }
}

async fn handle_query(
    socket: Arc<UdpSocket>,
    stats: Arc<DnsStats>,
    log: Arc<DnsLog>,
    query: Vec<u8>,
    client: SocketAddr,
    upstream: SocketAddr,
) {
    stats.queries_total.fetch_add(1, Ordering::Relaxed);

    // Header is 12 bytes; questions start at offset 12.
    let hostname: Option<String> = if query.len() >= 12 {
        parse_qname(&query, 12).map(|(h, _)| h)
    } else {
        None
    };

    // Check blocklist
    if let Some(h) = &hostname {
        if let Some((vendor, category)) = lookup_block(h) {
            stats.blocked_total.fetch_add(1, Ordering::Relaxed);
            log.push(DnsLogEntry {
                domain: h.clone(),
                blocked: true,
                vendor: Some(vendor.to_owned()),
                category: Some(category.to_owned()),
                timestamp_ms: now_ms(),
            })
            .await;
            tracing::debug!("DNS BLOCKED  {} ({})", h, vendor);
            let resp = build_nxdomain(&query);
            if !resp.is_empty() {
                let _ = socket.send_to(&resp, client).await;
            }
            return;
        }
    }

    // Forward to upstream DNS. The forwarding socket is `connect()`-ed to
    // `upstream` so the OS itself refuses to deliver any datagram whose
    // source address doesn't match — without this, an off-path attacker who
    // can reach this socket (it's bound on 0.0.0.0, not just loopback) could
    // race the real upstream with a spoofed response and have it forwarded
    // straight to the client, defeating the whole point of running a
    // "security" DNS resolver. See response_transaction_id_matches below
    // for the second half of this defense.
    let fwd = match UdpSocket::bind("0.0.0.0:0").await {
        Ok(s) => s,
        Err(e) => {
            tracing::warn!("DNS forward bind error: {}", e);
            return;
        }
    };
    if fwd.connect(upstream).await.is_err() {
        tracing::warn!("DNS forward connect error to {}", upstream);
        return;
    }

    if fwd.send(&query).await.is_err() {
        return;
    }

    let mut resp_buf = [0u8; 512];
    match tokio::time::timeout(std::time::Duration::from_secs(3), fwd.recv(&mut resp_buf)).await {
        Ok(Ok(n)) => {
            if response_transaction_id_matches(&query, &resp_buf[..n]) {
                let _ = socket.send_to(&resp_buf[..n], client).await;
            } else {
                tracing::warn!(
                    "DNS response transaction ID mismatch for {:?}, dropping",
                    hostname
                );
            }
        }
        _ => {
            tracing::debug!("DNS upstream timeout for {:?}", hostname);
        }
    }
}

/// Transaction ID (first 2 bytes of a DNS packet) must round-trip unchanged
/// between query and response — a second, cheap layer of defense against
/// response confusion/spoofing on top of the `connect()`-based source-address
/// filtering above.
fn response_transaction_id_matches(query: &[u8], response: &[u8]) -> bool {
    query.len() >= 2 && response.len() >= 2 && query[0] == response[0] && query[1] == response[1]
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_lookup_block_exact() {
        assert!(lookup_block("hotjar.com").is_some());
        assert!(lookup_block("HOTJAR.COM").is_some());
    }

    #[test]
    fn test_lookup_block_subdomain() {
        assert!(lookup_block("static.hotjar.com").is_some());
        assert!(lookup_block("cdn.amplitude.com").is_some());
        assert!(lookup_block("script.hotjar.com.").is_some()); // trailing dot
    }

    #[test]
    fn test_lookup_block_negative() {
        assert!(lookup_block("example.com").is_none());
        assert!(lookup_block("google.com").is_none()); // google.com itself is not blocked
        assert!(lookup_block("facebook.com").is_none()); // only connect.facebook.net
    }

    #[test]
    fn test_build_nxdomain_length() {
        // Minimal valid DNS query for "a.b" (11 bytes qname + 4 header bytes in question)
        // Header (12) + QNAME (5: \x01a\x01b\x00) + QTYPE (2) + QCLASS (2)
        let query = vec![
            0xAB, 0xCD, // ID
            0x01, 0x00, // Flags: standard query with RD=1
            0x00, 0x01, // QDCOUNT = 1
            0x00, 0x00, // ANCOUNT = 0
            0x00, 0x00, // NSCOUNT = 0
            0x00, 0x00, // ARCOUNT = 0
            0x01, b'a', // label "a"
            0x01, b'b', // label "b"
            0x00, // root label
            0x00, 0x01, // QTYPE = A
            0x00, 0x01, // QCLASS = IN
        ];
        let resp = build_nxdomain(&query);
        assert_eq!(resp[0], 0xAB); // ID preserved
        assert_eq!(resp[1], 0xCD);
        assert_eq!(resp[2] & 0x80, 0x80); // QR = 1
        assert_eq!(resp[3] & 0x0F, 3); // RCODE = NXDOMAIN
                                       // ANCOUNT should be 0
        assert_eq!(resp[6], 0);
        assert_eq!(resp[7], 0);
        let _ = query; // suppress unused warning
    }

    #[test]
    fn test_parse_qname_simple() {
        // Packet: header (12 bytes of zeros) + QNAME for "hotjar.com" + QTYPE + QCLASS
        let mut pkt = vec![0u8; 12];
        // \x06hotjar\x03com\x00
        pkt.extend_from_slice(&[6, b'h', b'o', b't', b'j', b'a', b'r']);
        pkt.extend_from_slice(&[3, b'c', b'o', b'm']);
        pkt.push(0x00); // root label
        pkt.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]); // QTYPE=A, QCLASS=IN
        let result = parse_qname(&pkt, 12);
        assert!(result.is_some());
        let (name, _) = result.unwrap();
        assert_eq!(name, "hotjar.com");
    }

    #[test]
    fn test_response_transaction_id_matches() {
        let query = [0xAB, 0xCD, 0, 0];
        let matching = [0xAB, 0xCD, 1, 1];
        let mismatched = [0xAB, 0xCE, 0, 0];
        assert!(response_transaction_id_matches(&query, &matching));
        assert!(!response_transaction_id_matches(&query, &mismatched));
        assert!(!response_transaction_id_matches(&query, &[0xAB]));
        assert!(!response_transaction_id_matches(&[0xAB], &matching));
    }

    /// This is the security property the DNS-spoofing fix in `handle_query`
    /// depends on: once the forwarding socket is `connect()`-ed to the real
    /// upstream, the OS itself must refuse to deliver a datagram from any
    /// other source, and must still deliver one from the real peer. Proven
    /// directly against real loopback sockets on this OS, not mocked —
    /// exactly the primitives `handle_query` actually uses.
    #[tokio::test]
    async fn connected_udp_socket_ignores_datagrams_from_other_sources() {
        let real_upstream = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = real_upstream.local_addr().unwrap();
        let attacker = UdpSocket::bind("127.0.0.1:0").await.unwrap();

        let fwd = UdpSocket::bind("0.0.0.0:0").await.unwrap();
        fwd.connect(upstream_addr).await.unwrap();
        let fwd_addr = fwd.local_addr().unwrap();

        // Attacker races a spoofed response straight at the forwarding
        // socket's address, from a source that is not the connected peer.
        attacker.send_to(b"spoofed", fwd_addr).await.unwrap();

        let mut buf = [0u8; 64];
        let result =
            tokio::time::timeout(std::time::Duration::from_millis(200), fwd.recv(&mut buf)).await;
        assert!(
            result.is_err(),
            "a connected UDP socket must not receive a datagram from a non-peer source"
        );

        // The real upstream's reply, from the address `fwd` is connected to,
        // must still be delivered.
        real_upstream.send_to(b"legit", fwd_addr).await.unwrap();
        let n = tokio::time::timeout(std::time::Duration::from_millis(200), fwd.recv(&mut buf))
            .await
            .expect("should not time out")
            .expect("recv should succeed");
        assert_eq!(&buf[..n], b"legit");
    }

    /// End-to-end through the real `handle_query` path (not just the raw
    /// socket primitive above): a fake upstream answers a non-blocked query
    /// and the client receives exactly that answer back.
    #[tokio::test]
    async fn handle_query_forwards_a_real_upstream_response_to_the_client() {
        let fake_upstream = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let upstream_addr = fake_upstream.local_addr().unwrap();

        let resolver_socket = Arc::new(UdpSocket::bind("127.0.0.1:0").await.unwrap());
        let client_socket = UdpSocket::bind("127.0.0.1:0").await.unwrap();
        let client_addr = client_socket.local_addr().unwrap();

        // Query for a non-blocked domain so handle_query takes the forward path.
        let mut query = vec![0xAB, 0xCD, 0x01, 0x00, 0x00, 0x01, 0, 0, 0, 0, 0, 0];
        query.extend_from_slice(&[7, b'e', b'x', b'a', b'm', b'p', b'l', b'e']);
        query.extend_from_slice(&[3, b'c', b'o', b'm', 0x00]);
        query.extend_from_slice(&[0x00, 0x01, 0x00, 0x01]);

        let stats = Arc::new(DnsStats {
            queries_total: AtomicU64::new(0),
            blocked_total: AtomicU64::new(0),
        });
        let log = Arc::new(DnsLog::new());

        let handler = tokio::spawn(handle_query(
            Arc::clone(&resolver_socket),
            stats,
            log,
            query.clone(),
            client_addr,
            upstream_addr,
        ));

        // Act as the real upstream: receive the forwarded query, reply with
        // a canned answer carrying the same transaction ID.
        let mut up_buf = [0u8; 512];
        let (n, from) = tokio::time::timeout(
            std::time::Duration::from_secs(1),
            fake_upstream.recv_from(&mut up_buf),
        )
        .await
        .expect("upstream should receive the forwarded query")
        .unwrap();
        assert_eq!(&up_buf[..n], &query[..]);

        let mut fake_answer = query.clone();
        fake_answer.extend_from_slice(b"FAKE_ANSWER");
        fake_upstream.send_to(&fake_answer, from).await.unwrap();

        handler.await.unwrap();

        let mut client_buf = [0u8; 512];
        let (n, _) = tokio::time::timeout(
            std::time::Duration::from_millis(500),
            client_socket.recv_from(&mut client_buf),
        )
        .await
        .expect("client should receive the resolver's reply")
        .unwrap();
        assert_eq!(&client_buf[..n], &fake_answer[..]);
    }
}
