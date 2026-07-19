use once_cell::sync::Lazy;
use prometheus::{
    register_counter, register_counter_vec, register_gauge, Counter, CounterVec, Encoder, Gauge,
    TextEncoder,
};

pub static REQUESTS_TOTAL: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_requests_total",
        "Total HTTP requests by endpoint and status",
        &["endpoint", "status"]
    )
    .unwrap()
});

pub static AUTH_FAILURES: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_auth_failures_total",
        "Authentication failures by reason",
        &["reason"]
    )
    .unwrap()
});

pub static CREDENTIALS_ISSUED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_credentials_issued_total",
        "Credentials issued by claim type",
        &["claim"]
    )
    .unwrap()
});

pub static CREDENTIALS_VERIFIED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_credentials_verified_total",
        "Credential verifications by result",
        &["result"]
    )
    .unwrap()
});

pub static AGENT_ANOMALIES: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_agent_anomalies_total",
        "AI agent anomaly events by type",
        &["event_type"]
    )
    .unwrap()
});

pub static ACTIVE_TENANTS: Lazy<Gauge> =
    Lazy::new(|| register_gauge!("hsip_active_tenants", "Number of active tenants").unwrap());

pub static MESSAGES_SIGNED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!("hsip_messages_signed_total", "Messages signed", &["tenant"]).unwrap()
});

pub static DECISIONS_RECORDED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_decisions_recorded_total",
        "AI-agent decision attestations recorded by decision_type",
        &["decision_type"]
    )
    .unwrap()
});

pub static DECISIONS_ANCHORED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_decisions_anchored_total",
        "Decision batches anchored by ots_status",
        &["ots_status"]
    )
    .unwrap()
});

pub static DECISIONS_VERIFIED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_decisions_verified_total",
        "Decision proof verifications by result",
        &["result"]
    )
    .unwrap()
});

/// Unauthenticated tenant/key creations via HSIP_SANDBOX=true. Watch this if
/// that env var is enabled anywhere it shouldn't be — it's the one endpoint
/// in the API that requires no bearer key at all.
pub static SANDBOX_PROVISIONS: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "hsip_sandbox_provisions_total",
        "Unauthenticated sandbox tenant provisions via POST /v1/sandbox/provision"
    )
    .unwrap()
});

/// Consecutive OpenTimestamps calendar submission failures across all
/// configured calendars in one anchor cycle. HSIP's decision-attestation
/// anchoring depends entirely on this external service; this metric is how
/// an operator notices that dependency has been degraded for a while
/// instead of only finding out via decisions.rs's ots_status='calendar_unreachable'
/// on individual anchors.
pub static ANCHOR_CALENDAR_UNREACHABLE: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "hsip_anchor_calendar_unreachable_total",
        "Anchor cycles where every configured OpenTimestamps calendar failed"
    )
    .unwrap()
});

/// Successful master key rotations. Should only ever move in small, rare,
/// deliberate increments — a rising rate would be very unexpected.
pub static MASTER_KEY_ROTATIONS: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "hsip_master_key_rotations_total",
        "Successful master key rotations via POST /v1/admin/master-key/rotate"
    )
    .unwrap()
});

/// Retries triggered by UNIQUE(tenant_id, prev_hash) conflicts in the
/// per-tenant hash chains (decisions, audit_entries). Near-zero at low
/// volume; a rising rate means a tenant's chain is under write contention.
pub static CHAIN_WRITE_RETRIES: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_chain_write_retries_total",
        "Hash-chain write retries after a UNIQUE(tenant_id, prev_hash) conflict, by chain",
        &["chain"]
    )
    .unwrap()
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
    Lazy::force(&DECISIONS_RECORDED);
    Lazy::force(&DECISIONS_ANCHORED);
    Lazy::force(&DECISIONS_VERIFIED);
    Lazy::force(&SANDBOX_PROVISIONS);
    Lazy::force(&ANCHOR_CALENDAR_UNREACHABLE);
    Lazy::force(&CHAIN_WRITE_RETRIES);
    Lazy::force(&MASTER_KEY_ROTATIONS);
}

/// Render all metrics as Prometheus text format
pub fn render() -> String {
    let encoder = TextEncoder::new();
    let families = prometheus::gather();
    let mut buf = Vec::new();
    encoder.encode(&families, &mut buf).unwrap_or_default();
    String::from_utf8(buf).unwrap_or_default()
}
