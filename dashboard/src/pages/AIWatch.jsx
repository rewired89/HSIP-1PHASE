import React, { useState, useEffect } from 'react';
import { request } from '../api';

function formatActivity(ms) {
  if (!ms) return 'No activity recorded';
  const ago  = Date.now() - ms;
  const mins = Math.floor(ago / 60_000);
  if (mins < 1)  return 'Just now';
  if (mins < 60) return `${mins} minute${mins !== 1 ? 's' : ''} ago`;
  const hours = Math.floor(mins / 60);
  if (hours < 24) return `${hours} hour${hours !== 1 ? 's' : ''} ago`;
  return `${Math.floor(hours / 24)} day${Math.floor(hours / 24) !== 1 ? 's' : ''} ago`;
}

function AgentCard({ agent, onRevoke, anomalous }) {
  const displayName = agent.name || `AI Agent · ${agent.key_id.slice(0, 8)}`;
  return (
    <div className={`agent-card${anomalous ? ' agent-card-anomalous' : ''}`}>
      <div className="agent-card-left">
        <div className="agent-icon">{anomalous ? '⚠️' : '🤖'}</div>
        <div>
          <div className="agent-name">{displayName}</div>
          <div className="agent-stats">
            <span>{agent.request_count ?? 0} request{agent.request_count !== 1 ? 's' : ''}</span>
            <span className="agent-dot">·</span>
            <span>Last active: {formatActivity(agent.window_start_ms)}</span>
            {anomalous && (
              <>
                <span className="agent-dot">·</span>
                <span className="agent-anomaly">
                  {agent.anomaly_count} anomaly flag{agent.anomaly_count !== 1 ? 's' : ''}
                </span>
              </>
            )}
          </div>
        </div>
      </div>
      <button className="danger" onClick={() => onRevoke(agent.key_id)}>
        Disconnect
      </button>
    </div>
  );
}

// ── Connect AI dialog ─────────────────────────────────────────────────────────

function ConnectDialog({ apiKey, onDone, onClose }) {
  const [name,      setName]      = useState('');
  const [expiry,    setExpiry]    = useState('never');
  const [busy,      setBusy]      = useState(false);
  const [newKey,    setNewKey]    = useState(null);   // shown after creation
  const [copied,    setCopied]    = useState(false);

  async function create() {
    if (!name.trim()) return;
    setBusy(true);
    try {
      const body = {
        name:       name.trim(),
        agent_type: 'ai_agent',
        expires_in_days: expiry === 'never' ? null : Number(expiry),
      };
      const res = await request('POST', '/v1/keys', body, apiKey);
      setNewKey(res.key);
      onDone();
    } catch (e) { alert(e.message); }
    setBusy(false);
  }

  function copyKey() {
    navigator.clipboard.writeText(newKey);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  if (newKey) {
    return (
      <div className="connect-dialog">
        <div className="connect-dialog-inner">
          <div className="connect-success-icon">✅</div>
          <h3>Connection created</h3>
          <p className="connect-warn">
            Copy this key now — it will <strong>never be shown again</strong>.
          </p>
          <div className="connect-key-box">
            <code>{newKey}</code>
            <button className="connect-copy-btn" onClick={copyKey}>
              {copied ? '✓ Copied' : 'Copy'}
            </button>
          </div>
          <p className="connect-hint">
            Paste this key into your AI system (ChatGPT plugin, custom script, etc.)
            as a <code>Bearer</code> token in the <code>Authorization</code> header.
            It will appear in your AI Watch list immediately after its first request.
          </p>
          <button className="primary" onClick={onClose} style={{ width: '100%', marginTop: '1rem' }}>
            Done
          </button>
        </div>
      </div>
    );
  }

  return (
    <div className="connect-dialog">
      <div className="connect-dialog-inner">
        <h3>Connect an AI system</h3>
        <p className="connect-hint">
          Give this connection a name so you remember what it is — e.g.
          "My ChatGPT", "Home automation", "Work assistant".
        </p>
        <label className="connect-label">Name</label>
        <input
          className="connect-input"
          placeholder="e.g. My ChatGPT plugin"
          value={name}
          onChange={e => setName(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && create()}
          autoFocus
        />
        <label className="connect-label">Expires</label>
        <select
          className="connect-input"
          value={expiry}
          onChange={e => setExpiry(e.target.value)}
        >
          <option value="never">Never</option>
          <option value="30">In 30 days</option>
          <option value="90">In 90 days</option>
          <option value="365">In 1 year</option>
        </select>
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

// ── Main component ────────────────────────────────────────────────────────────

export default function AIWatch({ apiKey }) {
  const [agents,      setAgents]      = useState([]);
  const [loading,     setLoading]     = useState(true);
  const [showConnect, setShowConnect] = useState(false);

  useEffect(() => {
    loadAgents();
    const interval = setInterval(loadAgents, 10_000);
    return () => clearInterval(interval);
  }, []);

  async function loadAgents() {
    try {
      const data = await request('GET', '/v1/agents', null, apiKey);
      setAgents(Array.isArray(data) ? data : []);
    } catch {}
    setLoading(false);
  }

  async function revokeAgent(keyId) {
    if (!confirm('Disconnect this AI? It will immediately lose access to your account.')) return;
    try {
      await request('DELETE', `/v1/keys/${keyId}`, null, apiKey);
      await loadAgents();
    } catch (e) { alert(e.message); }
  }

  const anomalous = agents.filter(a => (a.anomaly_count ?? 0) > 0);
  const normal    = agents.filter(a => (a.anomaly_count ?? 0) === 0);

  return (
    <div>
      {showConnect && (
        <ConnectDialog
          apiKey={apiKey}
          onDone={loadAgents}
          onClose={() => setShowConnect(false)}
        />
      )}

      <div className="consumer-hero">
        <div className="consumer-hero-icon">🤖</div>
        <h2>AI Watch</h2>
        <p>
          Control which AI systems can connect to your HSIP account.
          Give trusted AI tools a key so they can log consent and verify messages on your behalf —
          and cut off any connection in one click.
        </p>
      </div>

      {loading && (
        <div className="card">
          <p className="empty">Loading AI connections…</p>
        </div>
      )}

      {!loading && agents.length === 0 && (
        <div className="card">
          <div className="aiwatch-empty">
            <div className="aiwatch-empty-icon">🔌</div>
            <strong>No AI systems connected</strong>
            <p>
              You haven't connected any AI tools yet. Click below to give an AI system
              secure access to your HSIP account.
            </p>
            <button className="primary" style={{ marginTop: '1rem' }} onClick={() => setShowConnect(true)}>
              + Connect an AI
            </button>
          </div>
        </div>
      )}

      {anomalous.length > 0 && (
        <div className="card aiwatch-alert-card">
          <div className="aiwatch-alert-header">⚠️ Unusual Activity Detected</div>
          <p className="aiwatch-alert-body">
            {anomalous.length} AI connection{anomalous.length !== 1 ? 's have' : ' has'} triggered
            anomaly flags. Review and disconnect anything you don't recognise.
          </p>
          {anomalous.map(a => (
            <AgentCard key={a.key_id} agent={a} onRevoke={revokeAgent} anomalous />
          ))}
        </div>
      )}

      {normal.length > 0 && (
        <div className="card">
          <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center', marginBottom: '1rem' }}>
            <h2 style={{ margin: 0 }}>Connected AI Systems</h2>
            <button className="primary" onClick={() => setShowConnect(true)}>+ Connect an AI</button>
          </div>
          <p className="aiwatch-normal-note">
            All activity looks normal. You can disconnect any of these at any time.
          </p>
          {normal.map(a => (
            <AgentCard key={a.key_id} agent={a} onRevoke={revokeAgent} />
          ))}
        </div>
      )}

      <div className="card protection-card">
        <h2>What is HSIP protecting?</h2>
        <div className="protection-grid">
          <div className="protection-item">
            <span>🔑</span>
            <div>
              <strong>Access Control</strong>
              <p>Every AI connection requires a cryptographic key. No key = no access, period.</p>
            </div>
          </div>
          <div className="protection-item">
            <span>📋</span>
            <div>
              <strong>Full Audit Trail</strong>
              <p>Every request an AI makes is timestamped in a log you can inspect any time.</p>
            </div>
          </div>
          <div className="protection-item">
            <span>⚡</span>
            <div>
              <strong>Instant Revocation</strong>
              <p>Disconnect any AI immediately. Access stops in real time — no delays.</p>
            </div>
          </div>
          <div className="protection-item">
            <span>🚨</span>
            <div>
              <strong>Anomaly Detection</strong>
              <p>HSIP flags unusual request patterns automatically before they become a problem.</p>
            </div>
          </div>
        </div>
      </div>
    </div>
  );
}
