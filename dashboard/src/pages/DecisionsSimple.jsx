import React, { useState, useEffect } from 'react';
import { request } from '../api';

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
  if (mins < 60) return `${mins} minute${mins !== 1 ? 's' : ''} ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours} hour${hours !== 1 ? 's' : ''} ago`;
  return `${Math.floor(hours / 24)} day${Math.floor(hours / 24) !== 1 ? 's' : ''} ago`;
}

// ── One decision, plain-language by default, technical detail on demand ──────

function DecisionRow({ d, apiKey }) {
  const [open, setOpen]     = useState(false);
  const [proof, setProof]   = useState(null);
  const [loading, setLoading] = useState(false);
  const [verifyResult, setVerifyResult] = useState(null);
  const [verifying, setVerifying] = useState(false);
  const agentLabel = d.agent_name || 'Unknown connection';

  async function toggle() {
    if (!open && !proof) {
      setLoading(true);
      try { setProof(await request('GET', `/v1/decisions/${d.id}/proof`, null, apiKey)); }
      catch (e) { alert(e.message); }
      setLoading(false);
    }
    setOpen(o => !o);
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
      setVerifyResult(await request('POST', '/v1/decisions/verify', body, apiKey));
    } catch (e) { setVerifyResult({ error: e.message }); }
    setVerifying(false);
  }

  return (
    <div className="simple-decision-row">
      <div className="simple-decision-summary" onClick={toggle}>
        <span className="simple-decision-icon">📈</span>
        <div className="simple-decision-main">
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.5rem', flexWrap: 'wrap' }}>
            <strong>{d.decision_type.replace(/[._]/g, ' ')}</strong>
            <span
              title="Which connected agent recorded this decision"
              style={{
                fontSize: '0.7rem', padding: '0.1rem 0.5rem', borderRadius: '999px',
                background: '#2d3748', color: '#90cdf4', whiteSpace: 'nowrap',
              }}
            >
              🤖 {agentLabel}
            </span>
          </div>
          <span className="simple-decision-sub">{d.strategy_id} · {timeAgo(d.timestamp_iso)}</span>
        </div>
        <span className={`badge ${d.anchored ? 'granted' : 'pending'}`}>
          {d.anchored ? '✓ Locked & verified' : '🔒 Locked, finishing up…'}
        </span>
        <span className="simple-decision-toggle">{open ? '▲' : '▼'}</span>
      </div>
      {open && (
        <div className="simple-decision-detail">
          {loading && <p className="empty">Checking…</p>}
          {!loading && proof && (
            <>
              <p style={{ color: '#a0aec0', fontSize: '0.85rem', marginBottom: '0.75rem' }}>
                {proof.anchored
                  ? 'This decision is cryptographically signed and locked into a tamper-evident record. Nobody — including HSIP — can quietly change or delete it.'
                  : 'This decision is already signed. HSIP is still finishing the extra step that makes deletion detectable — check back shortly.'}
              </p>
              <button className="primary" onClick={verify} disabled={verifying}>
                {verifying ? 'Checking…' : "Double-check it's genuine"}
              </button>
              {verifyResult && (
                <div style={{ marginTop: '0.75rem', padding: '0.85rem', borderRadius: '6px',
                              background: verifyResult.error ? '#2d1b1b' : verifyResult.valid ? '#1a2d1a' : '#2d1b1b',
                              border: `1px solid ${verifyResult.valid ? '#68d391' : '#fc8181'}` }}>
                  <p style={{ color: verifyResult.valid ? '#68d391' : '#fc8181', fontWeight: 'bold', margin: 0 }}>
                    {verifyResult.error ? `Error: ${verifyResult.error}` : verifyResult.valid ? '✓ Genuine and untampered' : '✗ Something is wrong with this record'}
                  </p>
                </div>
              )}
              <details style={{ marginTop: '0.85rem' }}>
                <summary style={{ cursor: 'pointer', color: '#718096', fontSize: '0.8rem' }}>
                  Show technical proof (for developers)
                </summary>
                <pre style={{ background: '#1a202c', padding: '0.85rem', borderRadius: '6px', fontSize: '0.7rem',
                              overflowX: 'auto', color: '#90cdf4', marginTop: '0.5rem', maxHeight: 220 }}>
                  {JSON.stringify(proof, null, 2)}
                </pre>
              </details>
            </>
          )}
        </div>
      )}
    </div>
  );
}

// ── Connect dialog, plain-language wrapper around the same real flow ─────────

function ConnectSimpleDialog({ apiKey, identity, onDone, onClose }) {
  const [name, setName] = useState('');
  const [busy, setBusy] = useState(false);
  const [newKey, setNewKey] = useState(null);
  const [copied, setCopied] = useState(false);
  const [showCode, setShowCode] = useState(false);

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

  function copy() {
    navigator.clipboard.writeText(newKey);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  if (newKey) {
    const base = window.location.origin;
    const accountableKey = identity?.verify_key || '';
    const snippet =
`from hsip.client import HSIPClient

client = HSIPClient(api_key="${newKey}", base_url="${base}")
payload_hash = client.hash_payload(b"describe the decision here")
client.record_decision(
    accountable_key="${accountableKey}",
    model_version="predicta-v1",
    strategy_id="mean-reversion-1",
    decision_type="trade.order",
    payload_hash=payload_hash,
)`;
    return (
      <div className="connect-dialog">
        <div className="connect-dialog-inner" style={{ maxWidth: 560, maxHeight: '90vh', overflowY: 'auto' }}>
          <div className="connect-success-icon">✅</div>
          <h3>You're connected</h3>
          <p className="connect-hint">
            Send this key to whoever manages your trading bot's software (or Predicta's
            team) — it's how their system proves its decisions to HSIP.
          </p>
          <div className="connect-key-box">
            <code>{newKey}</code>
            <button className="connect-copy-btn" onClick={copy}>{copied ? '✓ Copied' : 'Copy'}</button>
          </div>
          <p className="connect-warn" style={{ marginTop: '1rem' }}>Save it now — it won't be shown again.</p>

          <button className="consumer-reset-btn" style={{ marginTop: '0.5rem' }} onClick={() => setShowCode(s => !s)}>
            {showCode ? 'Hide the code' : 'Show the code (for your developer)'}
          </button>
          {showCode && (
            <div className="setup-code-block" style={{ marginTop: '0.75rem' }}>
              <pre>{snippet}</pre>
            </div>
          )}

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
        <h3>Connect a trading bot</h3>
        <p className="connect-hint">
          Give it a name you'll recognise — e.g. <em>"Predicta"</em> or <em>"My broker"</em>.
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
            {busy ? 'Connecting…' : 'Connect'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Main page ──────────────────────────────────────────────────────────────────

const TIME_WINDOWS = {
  all: null,
  '24h': 24 * 60 * 60 * 1000,
  '7d':  7  * 24 * 60 * 60 * 1000,
  '30d': 30 * 24 * 60 * 60 * 1000,
};

export default function DecisionsSimple({ apiKey }) {
  const [identity, setIdentity] = useState(null);
  const [decisions, setDecisions] = useState([]);
  const [agents, setAgents] = useState([]);
  const [loading, setLoading] = useState(true);
  const [showConnect, setShowConnect] = useState(false);
  const [desc, setDesc] = useState('');
  const [recording, setRecording] = useState(false);
  const [justRecorded, setJustRecorded] = useState(false);
  const [filterAgent, setFilterAgent] = useState('all');
  const [filterWindow, setFilterWindow] = useState('all');

  useEffect(() => {
    loadIdentity();
    loadAgents();
  }, []);

  // Re-fetch whenever a filter changes, and keep polling on the current filter.
  useEffect(() => {
    loadDecisions();
    const id = setInterval(loadDecisions, 10_000);
    return () => clearInterval(id);
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [filterAgent, filterWindow]);

  async function loadIdentity() {
    try { setIdentity(await request('POST', '/v1/identity', null, apiKey)); } catch {}
  }

  async function loadAgents() {
    try {
      const data = await request('GET', '/v1/agents', null, apiKey);
      setAgents(Array.isArray(data) ? data : []);
    } catch {}
  }

  async function loadDecisions() {
    try {
      const params = new URLSearchParams();
      if (filterAgent !== 'all') params.set('agent_key_id', filterAgent);
      const windowMs = TIME_WINDOWS[filterWindow];
      if (windowMs) params.set('since_ms', String(Date.now() - windowMs));
      const qs = params.toString();
      const data = await request('GET', `/v1/decisions${qs ? `?${qs}` : ''}`, null, apiKey);
      setDecisions(Array.isArray(data) ? data : []);
    } catch {}
    setLoading(false);
  }

  async function tryIt() {
    if (!identity) { alert('Still getting ready — try again in a moment.'); return; }
    if (!desc.trim()) { alert('Describe a decision first, e.g. "Bought 100 shares of Apple".'); return; }
    setRecording(true);
    setJustRecorded(false);
    try {
      const payload_hash = await sha256Hex(desc);
      await request('POST', '/v1/decisions', {
        accountable_key: identity.verify_key,
        model_version:   'manual-test',
        strategy_id:      'try-it-yourself',
        decision_type:    'trade.order',
        payload_hash,
      }, apiKey);
      setDesc('');
      setJustRecorded(true);
      loadDecisions();
      setTimeout(() => setJustRecorded(false), 4000);
    } catch (e) { alert(e.message); }
    setRecording(false);
  }

  return (
    <div>
      {showConnect && (
        <ConnectSimpleDialog apiKey={apiKey} identity={identity} onDone={loadDecisions} onClose={() => setShowConnect(false)} />
      )}

      <div className="consumer-hero">
        <div className="consumer-hero-icon">📈</div>
        <h2>AI Decisions</h2>
        <p>
          Your AI trading systems make decisions — buy, sell, hold. HSIP keeps an
          unforgeable record of every one, the instant it happens, so you can always
          prove what happened and when — without having to trust HSIP's word for it.
        </p>
      </div>

      <div className="card">
        <h2>How it works</h2>
        <div className="setup-step">
          <div className="setup-step-num">1</div>
          <div className="setup-step-body">
            <strong>Your AI makes a decision</strong>
            <p>Predicta, a broker platform, or any trading bot decides to buy, sell, or hold.</p>
          </div>
        </div>
        <div className="setup-step">
          <div className="setup-step-num">2</div>
          <div className="setup-step-body">
            <strong>HSIP locks it in, instantly</strong>
            <p>It's digitally signed and time-stamped the moment it happens. HSIP never sees the actual trade — only proof that something specific was decided.</p>
          </div>
        </div>
        <div className="setup-step">
          <div className="setup-step-num">3</div>
          <div className="setup-step-body">
            <strong>Anyone you choose can check it's real</strong>
            <p>No need to take HSIP's word for it — the proof stands on its own.</p>
          </div>
        </div>
      </div>

      <div className="card">
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '0.5rem' }}>
          <h2 style={{ margin: 0 }}>Connect a trading bot</h2>
          <button className="primary" onClick={() => setShowConnect(true)}>+ Connect</button>
        </div>
        <p style={{ color: '#718096', fontSize: '0.85rem', marginTop: '0.5rem' }}>
          Connect Predicta, a broker platform, or any bot so its decisions show up here automatically.
        </p>
      </div>

      <div className="card">
        <h2>Try it yourself</h2>
        <p style={{ color: '#718096', fontSize: '0.85rem', marginBottom: '0.75rem' }}>
          Describe any decision in plain English — it never leaves your browser in readable form.
        </p>
        <div style={{ display: 'flex', gap: '0.75rem', flexWrap: 'wrap' }}>
          <input
            style={{ flex: 1, minWidth: 220, marginBottom: 0 }}
            placeholder='e.g. "Bought 100 shares of Apple at $191.20"'
            value={desc}
            onChange={e => setDesc(e.target.value)}
            onKeyDown={e => e.key === 'Enter' && tryIt()}
          />
          <button className="primary" onClick={tryIt} disabled={recording || !identity}>
            {recording ? 'Locking it in…' : 'Lock this decision in'}
          </button>
        </div>
        {justRecorded && (
          <p style={{ color: '#68d391', marginTop: '0.75rem' }}>✓ Done — see it below.</p>
        )}
      </div>

      <div className="card">
        <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', flexWrap: 'wrap', gap: '0.75rem' }}>
          <h2 style={{ margin: 0 }}>Recent decisions</h2>
          <div style={{ display: 'flex', gap: '0.5rem', flexWrap: 'wrap' }}>
            <select
              className="connect-input"
              style={{ marginBottom: 0, width: 'auto' }}
              value={filterAgent}
              onChange={e => setFilterAgent(e.target.value)}
            >
              <option value="all">All agents</option>
              {agents.map(a => (
                <option key={a.key_id} value={a.key_id}>{a.name}</option>
              ))}
            </select>
            <select
              className="connect-input"
              style={{ marginBottom: 0, width: 'auto' }}
              value={filterWindow}
              onChange={e => setFilterWindow(e.target.value)}
            >
              <option value="all">All time</option>
              <option value="24h">Last 24 hours</option>
              <option value="7d">Last 7 days</option>
              <option value="30d">Last 30 days</option>
            </select>
          </div>
        </div>
        {loading && <p className="empty">Loading…</p>}
        {!loading && decisions.length === 0 && (
          <p className="empty">
            {(filterAgent !== 'all' || filterWindow !== 'all')
              ? 'Nothing matches this filter — try widening it.'
              : 'Nothing yet — try the box above, or connect a trading bot.'}
          </p>
        )}
        <div className="simple-decision-list">
          {decisions.map(d => <DecisionRow key={d.id} d={d} apiKey={apiKey} />)}
        </div>
      </div>
    </div>
  );
}
