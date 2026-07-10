import React, { useState, useEffect } from 'react';
import { request } from '../api';

// SHA-256 in the browser — mirrors the design goal server-side: HSIP only
// ever receives a hash, never the actual decision content.
async function sha256Hex(text) {
  const bytes = new TextEncoder().encode(text);
  const digest = await crypto.subtle.digest('SHA-256', bytes);
  return Array.from(new Uint8Array(digest)).map(b => b.toString(16).padStart(2, '0')).join('');
}

function timeAgo(iso) {
  if (!iso) return '';
  const ms = Date.now() - new Date(iso).getTime();
  const mins = Math.floor(ms / 60_000);
  if (mins < 1) return 'just now';
  if (mins < 60) return `${mins}m ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours}h ago`;
  return `${Math.floor(hours / 24)}d ago`;
}

// ── Connect dialog: registers an ai_agent key + shows copy-paste snippets ─────

function ConnectDialog({ apiKey, identity, onDone, onClose }) {
  const [name, setName] = useState('');
  const [busy, setBusy] = useState(false);
  const [newKey, setNewKey] = useState(null);
  const [keyCopied, setKeyCopied] = useState(false);

  async function create() {
    if (!name.trim()) return;
    setBusy(true);
    try {
      const res = await request('POST', '/v1/keys', { name: name.trim(), agent_type: 'ai_agent' }, apiKey);
      setNewKey(res.key);
      onDone();
    } catch (e) { alert(e.message); }
    setBusy(false);
  }

  function copyKey() {
    navigator.clipboard.writeText(newKey);
    setKeyCopied(true);
    setTimeout(() => setKeyCopied(false), 2000);
  }

  if (newKey) {
    const base = window.location.origin;
    const accountableKey = identity?.verify_key || '<open this page once with an identity loaded>';

    const pythonSnippet =
`from hsip.client import HSIPClient

client = HSIPClient(api_key="${newKey}", base_url="${base}")

payload_hash = client.hash_payload(b"BUY 100 AAPL @ 191.20")
receipt = client.record_decision(
    accountable_key="${accountableKey}",
    model_version="predicta-v1",
    strategy_id="mean-reversion-1",
    decision_type="trade.order",
    payload_hash=payload_hash,
    receipt_dir="./receipts",   # keeps your own copy of the signed receipt
)`;

    const curlSnippet =
`curl -X POST ${base}/v1/decisions \\
  -H "Authorization: Bearer ${newKey}" \\
  -H "Content-Type: application/json" \\
  -d '{
    "accountable_key": "${accountableKey}",
    "model_version": "predicta-v1",
    "strategy_id": "mean-reversion-1",
    "decision_type": "trade.order",
    "payload_hash": "<sha256 hex of your real decision content>"
  }'`;

    return (
      <div className="connect-dialog">
        <div className="connect-dialog-inner" style={{ maxWidth: 620, maxHeight: '90vh', overflowY: 'auto' }}>
          <div className="connect-success-icon">✅</div>
          <h3>Connection created</h3>
          <p className="connect-warn">
            Copy this key now — it will <strong>never be shown again</strong>.
          </p>
          <div className="connect-key-box">
            <code>{newKey}</code>
            <button className="connect-copy-btn" onClick={copyKey}>
              {keyCopied ? '✓ Copied' : 'Copy'}
            </button>
          </div>

          <p className="connect-hint" style={{ marginTop: '1.25rem' }}>
            Give this key to Predicta — or any broker platform, bot, or trading system.
            HSIP never receives the actual decision content, only a SHA-256 hash the
            caller computes locally.
          </p>

          <div className="integration-steps">
            <div className="setup-step">
              <div className="setup-step-num">1</div>
              <div className="setup-step-body">
                <strong>Python (Predicta today) — using the HSIP SDK</strong>
                <div className="setup-code-block"><pre>{pythonSnippet}</pre></div>
              </div>
            </div>
            <div className="setup-step">
              <div className="setup-step-num">2</div>
              <div className="setup-step-body">
                <strong>Any other language — raw HTTP</strong>
                <p>No SDK yet for Node/Go/etc. — call the REST API directly the same way:</p>
                <div className="setup-code-block"><pre>{curlSnippet}</pre></div>
              </div>
            </div>
            <div className="setup-step">
              <div className="setup-step-num">3</div>
              <div className="setup-step-body">
                <strong>Check it worked</strong>
                <p>
                  It shows up in the Decisions list below within a few seconds, marked
                  <em> "pending anchor"</em> until the next anchor cycle runs (every ~5
                  minutes, or sooner once 50+ decisions pile up).
                </p>
              </div>
            </div>
          </div>

          <button className="primary" onClick={onClose} style={{ width: '100%', marginTop: '1.25rem' }}>
            Done
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="connect-dialog">
      <div className="connect-dialog-inner">
        <h3>Connect a trading system</h3>
        <p className="connect-hint">
          Name this connection so you know which system it belongs to —
          e.g. <em>"Predicta"</em>, <em>"IB Gateway bot"</em>, <em>"Broker X"</em>.
        </p>
        <label className="connect-label">Name</label>
        <input
          className="connect-input"
          placeholder="e.g. Predicta"
          value={name}
          onChange={e => setName(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && create()}
          autoFocus
        />
        <div className="connect-actions">
          <button className="consumer-reset-btn" onClick={onClose}>Cancel</button>
          <button className="primary" onClick={create} disabled={busy || !name.trim()}>
            {busy ? 'Creating…' : 'Create connection'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Proof panel: full bundle + independent verify, no DB trust required ───────

function ProofPanel({ decisionId, apiKey, onClose }) {
  const [proof, setProof] = useState(null);
  const [loading, setLoading] = useState(true);
  const [verifying, setVerifying] = useState(false);
  const [verifyResult, setVerifyResult] = useState(null);

  useEffect(() => { load(); }, [decisionId]);

  async function load() {
    setLoading(true);
    try { setProof(await request('GET', `/v1/decisions/${decisionId}/proof`, null, apiKey)); }
    catch (e) { alert(e.message); }
    setLoading(false);
  }

  async function verify() {
    if (!proof) return;
    setVerifying(true);
    try {
      const body = {
        envelope: proof.envelope,
        event_hash: proof.event_hash,
        signature: proof.signature,
        issuer_verify_key: proof.issuer_verify_key,
        ...(proof.merkle_root ? {
          merkle_root: proof.merkle_root,
          inclusion_proof: proof.inclusion_proof,
          anchor_signature: proof.anchor_signature,
          anchor_verify_key: proof.anchor_verify_key,
        } : {}),
      };
      // This calls HSIP's own /v1/decisions/verify — but note that endpoint
      // takes no auth and touches no DB; any third party could run the exact
      // same check themselves without HSIP at all.
      setVerifyResult(await request('POST', '/v1/decisions/verify', body, apiKey));
    } catch (e) { setVerifyResult({ error: e.message }); }
    setVerifying(false);
  }

  return (
    <div className="connect-dialog">
      <div className="connect-dialog-inner" style={{ maxWidth: 660, maxHeight: '90vh', overflowY: 'auto' }}>
        <h3>Decision Proof</h3>
        {loading && <p className="empty">Loading…</p>}
        {!loading && proof && (
          <>
            <p style={{ color: '#718096', fontSize: '0.85rem', marginBottom: '0.75rem' }}>
              Self-contained verification bundle — anyone can check this independently,
              with zero calls back to HSIP's database.
            </p>
            <span className={`badge ${proof.anchored ? 'granted' : 'pending'}`}>
              {proof.anchored ? 'Anchored' : 'Pending anchor'}
            </span>
            {!proof.anchored && (
              <p style={{ color: '#718096', fontSize: '0.78rem', marginTop: '0.5rem' }}>
                Authorship (signature) is already provable. Tamper-evidence over time
                (Merkle inclusion + external anchor) isn't yet — check back after the
                next anchor cycle.
              </p>
            )}
            <pre style={{ background: '#1a202c', padding: '1rem', borderRadius: '6px', fontSize: '0.72rem',
                          overflowX: 'auto', color: '#90cdf4', marginTop: '0.75rem', maxHeight: 280 }}>
              {JSON.stringify(proof, null, 2)}
            </pre>
            <button className="primary" onClick={verify} disabled={verifying} style={{ marginTop: '0.75rem' }}>
              {verifying ? 'Verifying…' : 'Verify independently'}
            </button>
            {verifyResult && (
              <div style={{ marginTop: '1rem', padding: '1rem', borderRadius: '6px',
                            background: verifyResult.error ? '#2d1b1b' : verifyResult.valid ? '#1a2d1a' : '#2d1b1b',
                            border: `1px solid ${verifyResult.valid ? '#68d391' : '#fc8181'}` }}>
                {verifyResult.error ? (
                  <p style={{ color: '#fc8181' }}>Error: {verifyResult.error}</p>
                ) : (
                  <>
                    <p style={{ color: verifyResult.valid ? '#68d391' : '#fc8181', fontWeight: 'bold', fontSize: '1.1rem' }}>
                      {verifyResult.valid ? '✓ VALID' : '✗ INVALID'}
                    </p>
                    {verifyResult.reason && (
                      <p style={{ color: '#a0aec0', fontSize: '0.85rem' }}>{verifyResult.reason}</p>
                    )}
                  </>
                )}
              </div>
            )}
          </>
        )}
        <button className="consumer-reset-btn" onClick={onClose} style={{ width: '100%', marginTop: '1.25rem' }}>
          Close
        </button>
      </div>
    </div>
  );
}

// ── Main page ──────────────────────────────────────────────────────────────────

export default function Decisions({ apiKey }) {
  const [identity, setIdentity]     = useState(null);
  const [decisions, setDecisions]   = useState([]);
  const [loading, setLoading]       = useState(true);
  const [showConnect, setShowConnect] = useState(false);
  const [proofFor, setProofFor]     = useState(null);

  const [desc, setDesc]             = useState('BUY 100 AAPL @ 191.20');
  const [modelVersion, setModelVersion] = useState('predicta-v1');
  const [strategyId, setStrategyId] = useState('mean-reversion-1');
  const [decisionType, setDecisionType] = useState('trade.order');
  const [recording, setRecording]   = useState(false);
  const [recorded, setRecorded]     = useState(null);

  useEffect(() => {
    loadIdentity();
    loadDecisions();
    const id = setInterval(loadDecisions, 10_000);
    return () => clearInterval(id);
  }, []);

  async function loadIdentity() {
    try { setIdentity(await request('POST', '/v1/identity', null, apiKey)); } catch {}
  }

  async function loadDecisions() {
    try {
      const data = await request('GET', '/v1/decisions', null, apiKey);
      setDecisions(Array.isArray(data) ? data : []);
    } catch {}
    setLoading(false);
  }

  async function recordTestDecision() {
    if (!identity) { alert('Identity still loading — try again in a moment.'); return; }
    if (!desc.trim()) { alert('Enter a decision description first.'); return; }
    setRecording(true);
    try {
      const payload_hash = await sha256Hex(desc);
      const r = await request('POST', '/v1/decisions', {
        accountable_key: identity.verify_key,
        model_version:   modelVersion,
        strategy_id:     strategyId,
        decision_type:   decisionType,
        payload_hash,
      }, apiKey);
      setRecorded(r);
      loadDecisions();
    } catch (e) { alert(e.message); }
    setRecording(false);
  }

  return (
    <div>
      {showConnect && (
        <ConnectDialog apiKey={apiKey} identity={identity} onDone={loadDecisions} onClose={() => setShowConnect(false)} />
      )}
      {proofFor && (
        <ProofPanel decisionId={proofFor} apiKey={apiKey} onClose={() => setProofFor(null)} />
      )}

      <div className="consumer-hero">
        <div className="consumer-hero-icon">📈</div>
        <h2>Decision Attestations</h2>
        <p>
          Every trading decision an AI agent makes gets signed, hash-chained, and
          anchored — cryptographic proof of who decided what and when, without HSIP
          ever seeing the actual trade. Connect Predicta, a broker platform, or any
          bot, and audit every bet it places.
        </p>
      </div>

      <div className="card">
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '0.5rem' }}>
          <h2 style={{ margin: 0 }}>Connect a trading system</h2>
          <button className="primary" onClick={() => setShowConnect(true)}>+ Connect a system</button>
        </div>
        <p style={{ color: '#718096', fontSize: '0.85rem', marginTop: '0.5rem' }}>
          Creates a dedicated key for Predicta (or any broker/bot) so its decisions are
          attributed to it specifically — with copy-paste Python and raw-HTTP snippets.
        </p>
      </div>

      <div className="card">
        <h2>Record a test decision</h2>
        <p style={{ color: '#718096', fontSize: '0.85rem', marginBottom: '0.75rem' }}>
          Try it yourself before wiring up a real system. The text below is hashed in
          your browser — HSIP only ever receives the hash, never the text itself.
        </p>
        <input
          placeholder="Decision description (e.g. BUY 100 AAPL @ 191.20)"
          value={desc}
          onChange={e => setDesc(e.target.value)}
        />
        <div style={{ display: 'grid', gridTemplateColumns: 'repeat(auto-fit, minmax(160px, 1fr))', gap: '0.75rem', marginTop: '0.75rem' }}>
          <input placeholder="Model version" value={modelVersion} onChange={e => setModelVersion(e.target.value)} />
          <input placeholder="Strategy ID" value={strategyId} onChange={e => setStrategyId(e.target.value)} />
          <input placeholder="Decision type" value={decisionType} onChange={e => setDecisionType(e.target.value)} />
        </div>
        <button className="primary" style={{ marginTop: '0.75rem' }} onClick={recordTestDecision} disabled={recording || !identity}>
          {recording ? 'Signing…' : 'Sign & Record Decision'}
        </button>

        {recorded && (
          <div style={{ marginTop: '1rem', padding: '1rem', borderRadius: '6px', background: '#1a2d1a', border: '1px solid #68d391' }}>
            <p style={{ color: '#68d391', fontWeight: 'bold' }}>✓ Signed and chained</p>
            <p style={{ color: '#a0aec0', fontSize: '0.8rem' }}>
              Decision ID: <code>{recorded.decision_id}</code>
            </p>
            <p style={{ color: '#a0aec0', fontSize: '0.8rem', wordBreak: 'break-all' }}>
              Event hash: <code>{recorded.event_hash}</code>
            </p>
          </div>
        )}
      </div>

      <div className="card">
        <h2>Decisions</h2>
        {loading && <p className="empty">Loading…</p>}
        {!loading && decisions.length === 0 && (
          <p className="empty">No decisions recorded yet. Record a test one above, or connect a real system.</p>
        )}
        {!loading && decisions.length > 0 && (
          <table>
            <thead>
              <tr>
                <th>Type</th>
                <th>Model</th>
                <th>Strategy</th>
                <th>Recorded</th>
                <th>Status</th>
                <th></th>
              </tr>
            </thead>
            <tbody>
              {decisions.map(d => (
                <tr key={d.id}>
                  <td><code style={{ fontSize: '0.8rem' }}>{d.decision_type}</code></td>
                  <td style={{ fontSize: '0.8rem' }}>{d.model_version}</td>
                  <td style={{ fontSize: '0.8rem' }}>{d.strategy_id}</td>
                  <td style={{ fontSize: '0.8rem' }} title={d.timestamp_iso}>{timeAgo(d.timestamp_iso)}</td>
                  <td>
                    <span className={`badge ${d.anchored ? 'granted' : 'pending'}`}>
                      {d.anchored ? 'anchored' : 'pending anchor'}
                    </span>
                  </td>
                  <td><button className="primary" onClick={() => setProofFor(d.id)}>View proof</button></td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
