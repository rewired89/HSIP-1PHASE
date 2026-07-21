use once_cell::sync::Lazy;
use prometheus::{
    register_counter, register_counter_vec, register_gauge, register_gauge_vec, Counter,
    CounterVec, Encoder, Gauge, GaugeVec, TextEncoder,
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

/// Deliberately *not* a `CounterVec` labeled by claim text, even though
/// that would read naturally at the call site. `claim` is arbitrary
/// caller-supplied free text (up to 64 chars, no other bound) — using it
/// as a Prometheus label value would (a) create one permanent time series
/// per unique claim string ever issued, an unbounded-cardinality leak that
/// only grows for the life of the process, and (b) publish the actual
/// claim content — potentially sensitive, e.g. a caller embedding a user
/// identifier — to `/metrics`, which has no authentication at all unless
/// an operator sets `METRICS_TOKEN`. Found during a QA pass asking "which
/// secret eventually becomes public" and "what is exposed during
/// debugging." A plain unlabeled counter still answers "how many
/// credentials have been issued," the only aggregate this metric is
/// actually used for.
pub static CREDENTIALS_ISSUED: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "hsip_credentials_issued_total",
        "Credentials issued (total count — not broken out by claim, which is arbitrary \
         caller-supplied free text unsafe to use as a metric label)"
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

/// Deliberately *not* labeled by `tenant_id`. A per-tenant label would
/// create one permanent Prometheus time series per tenant that has ever
/// signed a message — on a multi-tenant deployment (or with
/// `HSIP_SANDBOX=true`'s self-service trial provisioning) that's unbounded
/// growth for the life of the process, and it would enumerate every
/// tenant's UUID and message-signing activity to anyone reaching the
/// unauthenticated-by-default `/metrics` endpoint. Same class of bug as
/// `CREDENTIALS_ISSUED` above — found during the same QA pass.
pub static MESSAGES_SIGNED: Lazy<Counter> =
    Lazy::new(|| register_counter!("hsip_messages_signed_total", "Messages signed").unwrap());

/// Deliberately *not* labeled by `decision_type` — same reasoning as
/// `CREDENTIALS_ISSUED` above. `decision_type` is caller-supplied free
/// text (up to 64 chars) with no enum constraint anywhere in this
/// codebase, so nothing stops unbounded-cardinality growth here either.
pub static DECISIONS_RECORDED: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "hsip_decisions_recorded_total",
        "AI-agent decision attestations recorded (total count — not broken out by \
         decision_type, which is unconstrained caller-supplied text unsafe to use as a \
         metric label)"
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

/// Audit-log batches anchored by ots_status — twin of `DECISIONS_ANCHORED`
/// for `anchor_job::run_audit_anchor_cycle`.
pub static AUDIT_ANCHORED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_audit_anchored_total",
        "Audit-log entry batches anchored by ots_status",
        &["ots_status"]
    )
    .unwrap()
});

/// Requests rejected by the opt-in replay-protection check (x-hsip-timestamp
/// + x-hsip-nonce headers). Zero unless a caller opts in, since the headers
/// are entirely optional — see `auth.rs::check_replay_protection`.
pub static REPLAY_REJECTED: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_replay_rejected_total",
        "Requests rejected by opt-in replay protection, by reason",
        &["reason"]
    )
    .unwrap()
});

/// Grants/revocations of the root-admin flag (`api_keys.is_root_admin`).
/// Should only ever move in small, rare, deliberate increments — a rising
/// rate would be very unexpected, same as `MASTER_KEY_ROTATIONS`.
pub static ROOT_ADMIN_CHANGES: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_root_admin_changes_total",
        "Root-admin flag grants/revocations via POST /v1/admin/root-admins/*, by action",
        &["action"]
    )
    .unwrap()
});

/// Anchor batches (decisions or audit-log) upgraded from `ots_status =
/// 'pending'` to `'confirmed'` by `anchor_job::run_upgrade_cycle` — i.e. a
/// calendar reported the batch's Merkle root has since been included in a
/// mined Bitcoin block. A near-zero rate isn't itself a problem (Bitcoin
/// confirmation legitimately takes time), but a batch that's been `pending`
/// for a very long time without ever showing up here is worth an operator
/// noticing.
pub static ANCHOR_UPGRADED_TO_CONFIRMED: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "hsip_anchor_upgraded_to_confirmed_total",
        "Anchor batches upgraded from pending to Bitcoin-confirmed"
    )
    .unwrap()
});

/// Anchor batches that stopped being auto-polled for upgrade because they
/// exceeded `anchor_job::MAX_PENDING_UPGRADE_AGE_MS` (7 days) still at
/// `ots_status = 'pending'`. Should stay at zero in normal operation — real
/// confirmations land within hours. A rising count means calendars are
/// failing to confirm submissions long-term and is worth investigating; the
/// underlying anchor data is still intact either way, just no longer
/// auto-upgraded.
pub static ANCHOR_UPGRADE_STALE: Lazy<Counter> = Lazy::new(|| {
    register_counter!(
        "hsip_anchor_upgrade_stale_total",
        "Anchor batches that exceeded the max pending-upgrade age and stopped being auto-polled"
    )
    .unwrap()
});

/// Failed writes to `audit_entries` at "best-effort" call sites — see
/// `audit_log::record_best_effort`. `action` is always one of a small,
/// fixed set of hardcoded string literals passed by this codebase's own
/// call sites (`key.created`, `master_key.rotated`, etc.), never
/// caller-supplied free text, so it's safe as a label — unlike
/// `CREDENTIALS_ISSUED`'s `claim` or `MESSAGES_SIGNED`'s `tenant`, which
/// were not (see those metrics' doc comments). Should be zero in normal
/// operation; any nonzero value means an audit-trail entry is missing for
/// an operation that otherwise succeeded, worth investigating immediately.
pub static AUDIT_WRITE_FAILURES: Lazy<CounterVec> = Lazy::new(|| {
    register_counter_vec!(
        "hsip_audit_write_failures_total",
        "Best-effort audit log writes that failed after their underlying operation already \
         succeeded, by action",
        &["action"]
    )
    .unwrap()
});

/// Current count of unresolved `system_health::check` issues, by severity
/// (`critical`|`warning`). A gauge, not a counter — it reflects the state
/// as of the last check, so it correctly drops back to zero once an issue
/// resolves, unlike a monotonic counter which would stay "triggered"
/// forever. Refreshed on every `GET /v1/admin/system-health` call and by a
/// periodic background task (see `main.rs`) so this stays current even if
/// nobody's polling that endpoint — the whole point is that a business
/// running real Prometheus alerting can fire on `hsip_system_health_issues{severity="critical"} > 0`
/// without needing to poll HSIP's own API themselves.
pub static SYSTEM_HEALTH_ISSUES: Lazy<GaugeVec> = Lazy::new(|| {
    register_gauge_vec!(
        "hsip_system_health_issues",
        "Current count of unresolved system-health issues needing operator attention, by severity",
        &["severity"]
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
    Lazy::force(&REPLAY_REJECTED);
    Lazy::force(&ROOT_ADMIN_CHANGES);
    Lazy::force(&AUDIT_ANCHORED);
    Lazy::force(&ANCHOR_UPGRADED_TO_CONFIRMED);
    Lazy::force(&ANCHOR_UPGRADE_STALE);
    Lazy::force(&SYSTEM_HEALTH_ISSUES);
    Lazy::force(&AUDIT_WRITE_FAILURES);
}

/// Render all metrics as Prometheus text format
pub fn render() -> String {
    let encoder = TextEncoder::new();
    let families = prometheus::gather();
    let mut buf = Vec::new();
    encoder.encode(&families, &mut buf).unwrap_or_default();
    String::from_utf8(buf).unwrap_or_default()
}
