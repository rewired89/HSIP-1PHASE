//! Printable evidence report. All interpolated values are HTML-escaped and
//! the page carries a restrictive CSP, because it is opened from the
//! console's origin.

use hsip_proof::{EvidencePack, PackVerification};

fn esc(s: &str) -> String {
    s.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

fn pretty(iso: &str) -> String {
    if iso.len() >= 16 {
        format!("{} UTC", iso[..16].replace('T', " "))
    } else {
        iso.to_string()
    }
}

fn check_label(name: &str) -> &str {
    match name {
        "content_hash" => "Content is unaltered",
        "signature" => "Signed by the organization",
        "manifest_hash" => "Export manifest intact",
        "manifest_signature" => "Export signed by the organization",
        "manifest_matches_proofs" => "No records removed or added",
        "proofs_valid" => "Every record verifies",
        "single_issuer" => "One issuing key throughout",
        "hash_chain" => "Unbroken hash chain",
        other => other,
    }
}

fn short(h: &str) -> String {
    if h.len() > 16 {
        format!("{}…{}", &h[..8], &h[h.len() - 6..])
    } else {
        h.to_string()
    }
}

pub fn render(
    title: &str,
    pack: &EvidencePack,
    v: &PackVerification,
    fingerprint: &str,
    from_iso: &str,
    to_iso: &str,
) -> String {
    let status = if v.valid {
        ("VERIFIED", "ok")
    } else {
        ("NOT VERIFIED", "bad")
    };
    let mut checks = String::new();
    for c in &v.checks {
        let (label, cls) = match c.ok {
            Some(true) => ("PASS", "ok"),
            Some(false) => ("FAIL", "bad"),
            None => ("N/A", "na"),
        };
        checks.push_str(&format!(
            "<tr><td class=\"{cls}\">{label}</td><td>{}</td><td>{}</td></tr>",
            esc(check_label(&c.name)),
            esc(&c.detail)
        ));
    }
    let mut rows = String::new();
    for b in &pack.proofs {
        let e = &b.envelope;
        rows.push_str(&format!(
            "<tr><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td class=\"mono\">{}</td><td class=\"mono\">{}</td><td>{}</td></tr>",
            esc(&e.seq), esc(&pretty(&e.timestamp_iso)), esc(&e.actor), esc(&e.action), esc(&e.subject),
            esc(&short(&e.payload_hash)), esc(&short(&b.event_hash)),
            if b.anchor.is_some() { "yes" } else { "pending" }
        ));
    }
    let first = pack
        .proofs
        .first()
        .map(|b| b.envelope.timestamp_iso.clone())
        .unwrap_or_default();
    let last = pack
        .proofs
        .last()
        .map(|b| b.envelope.timestamp_iso.clone())
        .unwrap_or_default();
    format!(
        r#"<!doctype html><html lang="en"><head><meta charset="utf-8">
<meta http-equiv="Content-Security-Policy" content="default-src 'none'; style-src 'unsafe-inline'">
<meta name="viewport" content="width=device-width,initial-scale=1">
<title>{title} · Evidence report</title>
<style>
body{{font:14px/1.5 -apple-system,Segoe UI,Roboto,sans-serif;color:#14181f;margin:0;padding:40px;max-width:1040px;margin:auto}}
h1{{font-size:24px;margin:0 0 4px}} h2{{font-size:15px;margin:28px 0 8px;text-transform:uppercase;letter-spacing:.06em;color:#4b5563}}
.sub{{color:#4b5563;margin:0 0 20px}} .banner{{padding:14px 18px;border-radius:8px;font-weight:600;margin:16px 0}}
.banner.ok{{background:#e8f6ee;color:#14633a;border:1px solid #b7e1c8}} .banner.bad{{background:#fdecec;color:#8a1c1c;border:1px solid #f2b8b8}}
.grid{{display:grid;grid-template-columns:repeat(4,1fr);gap:12px}} .card{{border:1px solid #d7dce3;border-radius:8px;padding:10px 14px}}
.card b{{display:block;font-size:17px}} .card span{{color:#4b5563;font-size:12px}}
table{{border-collapse:collapse;width:100%;font-size:12.5px}} th,td{{border-bottom:1px solid #e5e8ed;padding:6px 8px;text-align:left;vertical-align:top}}
th{{color:#4b5563;font-weight:600}} .mono{{font-family:ui-monospace,Menlo,Consolas,monospace;font-size:11.5px}}
td.ok{{color:#14633a;font-weight:600}} td.bad{{color:#8a1c1c;font-weight:600}} td.na{{color:#6b7280}}
.key{{word-break:break-all}} .note{{color:#4b5563;font-size:12.5px;border-top:1px solid #e5e8ed;margin-top:28px;padding-top:12px}}
@media print{{body{{padding:0}}}}
</style></head><body>
<h1>{title}</h1>
<p class="sub">{org} · stream “{stream}” · generated {generated}</p>
<div class="banner {cls}">{status}: {valid_text}</div>
<div class="grid">
<div class="card"><b>{count}</b><span>records</span></div>
<div class="card"><b>{anchored}</b><span>anchored in a signed batch</span></div>
<div class="card"><b>{first}</b><span>first record</span></div>
<div class="card"><b>{last}</b><span>last record</span></div>
</div>
<h2>Scope</h2>
<p>Records created from <b>{from}</b> to <b>{to}</b>.</p>
<h2>Integrity checks</h2>
<table><tr><th>Result</th><th>Check</th><th>Detail</th></tr>{checks}</table>
<h2>Issuer</h2>
<p>Organization key fingerprint <span class="mono">{fp}</span><br><span class="mono key">{vk}</span></p>
<p>Manifest hash <span class="mono key">{mh}</span></p>
<h2>Records</h2>
<table><tr><th>#</th><th>Time (UTC)</th><th>Actor</th><th>Action</th><th>Subject</th><th>Payload hash</th><th>Event hash</th><th>Anchored</th></tr>{rows}</table>
<div class="note"><b>Verify without trusting us.</b> Download the evidence pack (JSON) and run <span class="mono">hsip-proof verify pack.json</span>. The verifier needs no account and no network. It recomputes every hash, checks every signature and Merkle path, and confirms the records form an unbroken chain.<br><br>
<b>What this proves.</b> That each record existed in this form, was signed by the key above, and has not been altered, removed or reordered since. It does not prove that the underlying decision was correct, and the original content is not held by the platform: only its SHA-256 hash is. To prove a specific record, hash the original and compare it with the payload hash.</div>
</body></html>"#,
        title = esc(title),
        org = esc(&pack.org_name),
        stream = esc(&pack.stream_name),
        generated = esc(&pretty(&pack.generated_at_iso)),
        cls = status.1,
        status = status.0,
        valid_text = if v.valid {
            "every applicable check passed"
        } else {
            "one or more checks failed, see below"
        },
        count = v.proof_count,
        anchored = v.anchored_count,
        first = esc(&pretty(&first)),
        last = esc(&pretty(&last)),
        from = esc(&pretty(from_iso)),
        to = esc(&pretty(to_iso)),
        checks = checks,
        fp = esc(fingerprint),
        vk = esc(&pack.org_verify_key),
        mh = esc(&pack.manifest_hash),
        rows = rows,
    )
}
