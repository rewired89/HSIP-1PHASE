use once_cell::sync::Lazy;
use prometheus::{
    register_counter_vec, register_gauge, CounterVec, Gauge, TextEncoder, Encoder,
};

pub static REQUESTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_requests_total",
        "Total HTTP requests by endpoint and status",
        &["endpoint", "status"]
    ).unwrap()
});

pub static AUTH_FAILURES: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_auth_failures_total",
        "Authentication failures by reason",
        &["reason"]
    ).unwrap()
});

pub static CREDENTIALS_ISSUED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_credentials_issued_total",
        "Credentials issued by claim type",
        &["claim"]
    ).unwrap()
});

pub static CREDENTIALS_VERIFIED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_credentials_verified_total",
        "Credential verifications by result",
        &["result"]
    ).unwrap()
});

pub static AGENT_ANOMALIES: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_agent_anomalies_total",
        "AI agent anomaly events by type",
        &["event_type"]
    ).unwrap()
});

pub static ACTIVE_TENANTS: Lazy<Gauge> = Lazy::new(|| {
    register_gauge!(
        "hsip_active_tenants",
        "Number of active tenants"
    ).unwrap()
});

pub static MESSAGES_SIGNED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_messages_signed_total",
        "Messages signed",
        &["tenant"]
    ).unwrap()
});

/// Force initialization of all metrics at startup
pub fn init() {
    Lazy::force(&REQUESTS_TOTAL);
    Lazy::force(&AUTH_FAILURES);
    Lazy::force(&CREDENTIALS_ISSUED);
    Lazy::force(&CREDENTIALS_VERIFIED);
    Lazy::force(&AGENT_ANOMALIES);
    Lazy::force(&ACTIVE_TENANTS);
    Lazy::force(&MESSAGES_SIGNED);
}

/// Render all metrics as Prometheus text format
pub fn render() -> String {
    let encoder = TextEncoder::new();
    let families = prometheus::gather();
    let mut buf = Vec::new();
    encoder.encode(&families, &mut buf).unwrap_or_default();
    String::from_utf8(buf).unwrap_or_default()
}
