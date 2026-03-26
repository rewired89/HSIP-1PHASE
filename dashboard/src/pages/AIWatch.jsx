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
  const displayName = agent.name || `AI Agent · ${agent.id.slice(0, 8)}`;

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
      <button className="danger" onClick={() => onRevoke(agent.id)}>
        Disconnect
      </button>
    </div>
  );
}

export default function AIWatch({ apiKey }) {
  const [agents,  setAgents]  = useState([]);
  const [loading, setLoading] = useState(true);

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
      <div className="consumer-hero">
        <div className="consumer-hero-icon">🤖</div>
        <h2>AI Watch</h2>
        <p>
          AI systems are connected to your account right now. Some you put there.
          Some you might not remember. This is your live feed —
          see exactly what's running, how active it is, and shut anything down
          in one click.
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
            <div className="aiwatch-empty-icon">✅</div>
            <strong>No AI systems connected</strong>
            <p>Your account has no active AI agent connections right now.</p>
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
            <AgentCard key={a.id} agent={a} onRevoke={revokeAgent} anomalous />
          ))}
        </div>
      )}

      {normal.length > 0 && (
        <div className="card">
          <h2>Connected AI Systems</h2>
          <p className="aiwatch-normal-note">
            All activity looks normal. You can disconnect any of these at any time.
          </p>
          {normal.map(a => (
            <AgentCard key={a.id} agent={a} onRevoke={revokeAgent} />
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
