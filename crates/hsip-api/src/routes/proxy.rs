//! HTTP/HTTPS proxy control and live traffic log.
//!
//! The proxy runs as a blocking OS thread that accepts connections on
//! 127.0.0.1:8877 (default).  Traffic events are pushed into the shared ring
//! buffer in `AppState.proxy` and served to the dashboard via `GET /v1/proxy/log`.
//!
//! Endpoints
//! ---------
//! GET  /v1/proxy/status  → ProxyStatus
//! POST /v1/proxy/enable  → ProxyStatus
//! POST /v1/proxy/disable → { ok }
//! GET  /v1/proxy/log     → Vec<ProxyEvent>
//! GET  /v1/proxy/setup   → SetupInstructions  (platform-specific)

use axum::{extract::State, http::StatusCode, response::IntoResponse, Json};
use serde::{Deserialize, Serialize};
use std::collections::VecDeque;
use std::io::{Read, Write};
use std::net::{SocketAddr, TcpListener, TcpStream, ToSocketAddrs};
use std::sync::atomic::Ordering;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use uuid::Uuid;

use crate::state::{AppState, ProxyEvent};

// ── Embedded tracker database ────────────────────────────────────────────────

/// Returns the category of a known tracker domain, or None if not a tracker.
fn tracker_category(host: &str) -> Option<&'static str> {
    let h = host.trim_start_matches("www.");
    // Remove port if present
    let h = h.split(':').next().unwrap_or(h);

    // Walk the suffix list: exact match or subdomain of a known entry.
    for (domain, cat) in TRACKERS {
        if h == *domain || h.ends_with(&format!(".{domain}")) {
            return Some(cat);
        }
    }
    None
}

/// (domain, category) — embedded, no config file needed.
static TRACKERS: &[(&str, &str)] = &[
    // Advertising
    ("doubleclick.net", "advertising"),
    ("googlesyndication.com", "advertising"),
    ("googleadservices.com", "advertising"),
    ("ads.google.com", "advertising"),
    ("adservice.google.com", "advertising"),
    ("pagead2.googlesyndication.com", "advertising"),
    ("amazon-adsystem.com", "advertising"),
    ("adsrvr.org", "advertising"),
    ("rubiconproject.com", "advertising"),
    ("pubmatic.com", "advertising"),
    ("openx.net", "advertising"),
    ("outbrain.com", "advertising"),
    ("taboola.com", "advertising"),
    ("criteo.com", "advertising"),
    ("criteo.net", "advertising"),
    ("adnxs.com", "advertising"),
    ("advertising.com", "advertising"),
    ("adsafeprotected.com", "advertising"),
    ("moatads.com", "advertising"),
    ("scorecardresearch.com", "advertising"),
    ("casalemedia.com", "advertising"),
    ("33across.com", "advertising"),
    ("smartadserver.com", "advertising"),
    ("serving-sys.com", "advertising"),
    ("yieldmo.com", "advertising"),
    ("sharethrough.com", "advertising"),
    ("spotxchange.com", "advertising"),
    ("appnexus.com", "advertising"),
    ("lijit.com", "advertising"),
    ("revcontent.com", "advertising"),
    // Analytics
    ("google-analytics.com", "analytics"),
    ("analytics.google.com", "analytics"),
    ("googletagmanager.com", "analytics"),
    ("googletagservices.com", "analytics"),
    ("mixpanel.com", "analytics"),
    ("segment.io", "analytics"),
    ("segment.com", "analytics"),
    ("amplitude.com", "analytics"),
    ("hotjar.com", "analytics"),
    ("mouseflow.com", "analytics"),
    ("fullstory.com", "analytics"),
    ("heap.io", "analytics"),
    ("intercom.io", "analytics"),
    ("intercom.com", "analytics"),
    ("kissmetrics.com", "analytics"),
    ("loggly.com", "analytics"),
    ("newrelic.com", "analytics"),
    ("nr-data.net", "analytics"),
    ("datadog-browser-agent.com", "analytics"),
    ("browser-intake-datadoghq.com", "analytics"),
    ("bugsnag.com", "analytics"),
    ("sentry.io", "analytics"),
    ("rollbar.com", "analytics"),
    ("logrocket.com", "analytics"),
    ("clarity.ms", "analytics"),
    ("crazyegg.com", "analytics"),
    ("optimizely.com", "analytics"),
    ("statsig.com", "analytics"),
    ("launchdarkly.com", "analytics"),
    // Social / tracking pixels
    ("facebook.com", "social"),
    ("connect.facebook.net", "social"),
    ("graph.facebook.com", "social"),
    ("pixel.facebook.com", "social"),
    ("twitter.com", "social"),
    ("t.co", "social"),
    ("analytics.twitter.com", "social"),
    ("linkedin.com", "social"),
    ("snap.licdn.com", "social"),
    ("tiktok.com", "social"),
    ("ads.tiktok.com", "social"),
    ("pinterest.com", "social"),
    ("ct.pinterest.com", "social"),
    ("reddit.com", "social"),
    ("alb.reddit.com", "social"),
    // Fingerprinting / data brokers
    ("fingerprintjs.com", "fingerprinting"),
    ("iovation.com", "fingerprinting"),
    ("threatmetrix.com", "fingerprinting"),
    ("forter.com", "fingerprinting"),
    ("bidswitch.net", "fingerprinting"),
    ("quantserve.com", "fingerprinting"),
    ("agkn.com", "fingerprinting"),
    ("bluekai.com", "fingerprinting"),
    ("demdex.net", "fingerprinting"),
    ("adobedc.net", "fingerprinting"),
    // Telemetry / crash reporting
    ("crashlytics.com", "telemetry"),
    ("firebase.com", "telemetry"),
    ("firebaseio.com", "telemetry"),
    ("app-measurement.com", "telemetry"),
    ("appsflyer.com", "telemetry"),
    ("branch.io", "telemetry"),
    ("adjust.com", "telemetry"),
    ("kochava.com", "telemetry"),
    ("singular.net", "telemetry"),
    ("mparticle.com", "telemetry"),
];

// ── Handlers ─────────────────────────────────────────────────────────────────

#[derive(Serialize)]
pub struct ProxyStatus {
    pub enabled: bool,
    pub port: u16,
    pub stats: ProxyStats,
}

#[derive(Serialize)]
pub struct ProxyStats {
    pub total: usize,
    pub blocked: usize,
    pub allowed: usize,
}

pub async fn status(State(s): State<AppState>) -> impl IntoResponse {
    let enabled = s.proxy.enabled.load(Ordering::Relaxed);
    let port = s.proxy.port.load(Ordering::Relaxed) as u16;
    let stats = compute_stats(&s);
    Json(ProxyStatus {
        enabled,
        port,
        stats,
    })
}

pub async fn enable(State(s): State<AppState>) -> impl IntoResponse {
    if s.proxy.enabled.load(Ordering::Relaxed) {
        let port = s.proxy.port.load(Ordering::Relaxed) as u16;
        let stats = compute_stats(&s);
        return (
            StatusCode::OK,
            Json(ProxyStatus {
                enabled: true,
                port,
                stats,
            }),
        );
    }

    let port = s.proxy.port.load(Ordering::Relaxed) as u16;
    let addr = format!("127.0.0.1:{}", port);

    let (shutdown_tx, shutdown_rx) = std::sync::mpsc::sync_channel::<()>(1);
    *s.proxy.shutdown.lock().unwrap() = Some(shutdown_tx);
    s.proxy.enabled.store(true, Ordering::Relaxed);

    let proxy_shared = s.proxy.clone();
    std::thread::spawn(move || {
        run_proxy_thread(addr, proxy_shared, shutdown_rx);
    });

    let stats = compute_stats(&s);
    (
        StatusCode::OK,
        Json(ProxyStatus {
            enabled: true,
            port,
            stats,
        }),
    )
}

pub async fn disable(State(s): State<AppState>) -> impl IntoResponse {
    // Send shutdown signal (non-blocking; thread reads it next accept cycle)
    if let Ok(mut guard) = s.proxy.shutdown.lock() {
        drop(guard.take());
    }
    s.proxy.enabled.store(false, Ordering::Relaxed);
    Json(serde_json::json!({ "ok": true }))
}

pub async fn log(State(s): State<AppState>) -> impl IntoResponse {
    let events: Vec<ProxyEvent> = s.proxy.events.lock().unwrap().iter().cloned().collect();
    Json(events)
}

#[derive(Serialize)]
pub struct SetupInstructions {
    pub proxy_host: String,
    pub proxy_port: u16,
    pub steps_windows: Vec<String>,
    pub steps_mac: Vec<String>,
    pub steps_browser: Vec<String>,
    pub pac_url: String,
}

pub async fn setup(State(s): State<AppState>) -> impl IntoResponse {
    let port = s.proxy.port.load(Ordering::Relaxed) as u16;
    Json(SetupInstructions {
        proxy_host: "127.0.0.1".into(),
        proxy_port: port,
        steps_windows: vec![
            "Open Windows Settings → Network & Internet → Proxy".into(),
            "Under Manual proxy setup, toggle Use a proxy server ON".into(),
            format!("Set Address to 127.0.0.1 and Port to {}", port),
            "Click Save. Your browser will now route through HSIP.".into(),
        ],
        steps_mac: vec![
            "Open System Settings → Network → select your connection → Details".into(),
            "Click the Proxies tab".into(),
            format!("Enable Web Proxy (HTTP) and Secure Web Proxy (HTTPS), set server to 127.0.0.1 port {}", port),
            "Click OK and Apply.".into(),
        ],
        steps_browser: vec![
            "In Chrome/Edge: Settings → System → Open your computer's proxy settings (then follow Windows/Mac steps above)".into(),
            format!("In Firefox: Settings → search 'proxy' → Manual proxy → HTTP: 127.0.0.1 port {} → check 'Also use for HTTPS'", port),
        ],
        pac_url: format!("http://127.0.0.1:7474/proxy.pac"),
    })
}

// ── Proxy thread ──────────────────────────────────────────────────────────────

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

fn push_event(shared: &crate::state::ProxyShared, ev: ProxyEvent) {
    let mut ring = shared.events.lock().unwrap();
    if ring.len() >= 500 {
        ring.pop_front();
    }
    ring.push_back(ev);
}

fn run_proxy_thread(
    addr: String,
    shared: std::sync::Arc<crate::state::ProxyShared>,
    shutdown: std::sync::mpsc::Receiver<()>,
) {
    let listener = match TcpListener::bind(&addr) {
        Ok(l) => l,
        Err(e) => {
            eprintln!("[proxy] bind {} failed: {}", addr, e);
            shared.enabled.store(false, Ordering::Relaxed);
            return;
        }
    };
    listener.set_nonblocking(true).ok();
    eprintln!("[proxy] listening on {}", addr);

    loop {
        // Check for shutdown
        if shutdown.try_recv().is_ok() {
            break;
        }
        // Check if disabled from outside
        if !shared.enabled.load(Ordering::Relaxed) {
            break;
        }

        match listener.accept() {
            Ok((client, peer)) => {
                let sh2 = shared.clone();
                std::thread::spawn(move || {
                    handle_connection(client, peer, sh2);
                });
            }
            Err(ref e) if e.kind() == std::io::ErrorKind::WouldBlock => {
                std::thread::sleep(Duration::from_millis(50));
            }
            Err(e) => {
                eprintln!("[proxy] accept error: {}", e);
                std::thread::sleep(Duration::from_millis(200));
            }
        }
    }
    eprintln!("[proxy] stopped");
    shared.enabled.store(false, Ordering::Relaxed);
}

fn handle_connection(
    mut client: TcpStream,
    _peer: SocketAddr,
    shared: std::sync::Arc<crate::state::ProxyShared>,
) {
    client
        .set_read_timeout(Some(Duration::from_millis(3000)))
        .ok();
    client
        .set_write_timeout(Some(Duration::from_millis(3000)))
        .ok();

    // Read request headers
    let mut req = Vec::new();
    let mut buf = [0u8; 4096];
    loop {
        match client.read(&mut buf) {
            Ok(0) | Err(_) => break,
            Ok(n) => {
                req.extend_from_slice(&buf[..n]);
                if req.windows(4).any(|w| w == b"\r\n\r\n") {
                    break;
                }
                if req.len() > 64 * 1024 {
                    return;
                }
            }
        }
    }
    if req.is_empty() {
        return;
    }

    let req_str = String::from_utf8_lossy(&req);
    let first = req_str.lines().next().unwrap_or("");
    let parts: Vec<&str> = first.splitn(3, ' ').collect();
    if parts.len() < 2 {
        return;
    }

    let method = parts[0].to_string();
    let target = parts[1].to_string();

    // Extract host
    let host = if method.eq_ignore_ascii_case("CONNECT") {
        target.split(':').next().unwrap_or(&target).to_string()
    } else {
        req_str
            .lines()
            .find(|l| l.to_ascii_lowercase().starts_with("host:"))
            .and_then(|l| l.splitn(2, ':').nth(1))
            .map(|h| h.trim().split(':').next().unwrap_or("").to_string())
            .unwrap_or_default()
    };

    let path = if method.eq_ignore_ascii_case("CONNECT") {
        String::new()
    } else {
        let url = target
            .trim_start_matches("http://")
            .trim_start_matches("https://");
        url.find('/')
            .map(|i| url[i..].to_string())
            .unwrap_or_else(|| "/".to_string())
    };

    // Classify
    let category = tracker_category(&host).map(|s| s.to_string());
    let blocked = category.is_some();

    let ev = ProxyEvent {
        id: Uuid::new_v4().to_string(),
        ts_ms: now_ms(),
        host: host.clone(),
        method: method.clone(),
        path: path.clone(),
        verdict: if blocked { "blocked" } else { "allowed" }.to_string(),
        category: category.clone(),
        reason: category.as_ref().map(|c| format!("tracker:{}", c)),
    };
    push_event(&shared, ev);

    if blocked {
        let body = format!(
            "<html><body style='font-family:sans-serif;padding:2rem'>\
            <h2 style='color:#e53e3e'>🛡 HSIP blocked this request</h2>\
            <p><strong>{}</strong> is a tracker ({}).</p>\
            <p>Your privacy is protected.</p></body></html>",
            html_escape(&host),
            html_escape(category.as_deref().unwrap_or(""))
        );
        let resp = format!(
            "HTTP/1.1 403 Forbidden\r\nContent-Type: text/html; charset=utf-8\r\n\
             Content-Length: {}\r\nConnection: close\r\n\r\n{}",
            body.len(),
            body
        );
        let _ = client.write_all(resp.as_bytes());
        return;
    }

    // Pass through
    if method.eq_ignore_ascii_case("CONNECT") {
        tunnel_connect(target, client);
    } else {
        relay_http(host, req, client);
    }
}

fn tunnel_connect(target: String, mut client: TcpStream) {
    let addr = match resolve(&target) {
        Ok(a) => a,
        Err(_) => return,
    };
    let mut server = match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
        Ok(s) => s,
        Err(_) => return,
    };
    let _ = client.write_all(b"HTTP/1.1 200 Connection Established\r\n\r\n");
    let _ = client.flush();
    let mut s2 = server.try_clone().unwrap();
    let mut c2 = client.try_clone().unwrap();
    std::thread::spawn(move || {
        let _ = std::io::copy(&mut c2, &mut s2);
    });
    let _ = std::io::copy(&mut server, &mut client);
}

fn relay_http(host: String, req: Vec<u8>, mut client: TcpStream) {
    let addr = match resolve(&format!("{}:80", host)) {
        Ok(a) => a,
        Err(_) => return,
    };
    let mut server = match TcpStream::connect_timeout(&addr, Duration::from_secs(5)) {
        Ok(s) => s,
        Err(_) => return,
    };
    let _ = server.write_all(&req);
    let _ = std::io::copy(&mut server, &mut client);
}

fn resolve(target: &str) -> std::io::Result<SocketAddr> {
    target
        .to_socket_addrs()?
        .next()
        .ok_or_else(|| std::io::Error::new(std::io::ErrorKind::NotFound, "no address"))
}

fn html_escape(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
}

// ── Helpers ──────────────────────────────────────────────────────────────────

fn compute_stats(s: &AppState) -> ProxyStats {
    let ring = s.proxy.events.lock().unwrap();
    let total = ring.len();
    let blocked = ring.iter().filter(|e| e.verdict == "blocked").count();
    ProxyStats {
        total,
        blocked,
        allowed: total - blocked,
    }
}
