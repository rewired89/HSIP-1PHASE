import React, { useState, useEffect } from 'react';
import { request } from '../api';

const BASE_URL = 'http://127.0.0.1:7777';

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

function CopyBox({ value, label }) {
  const [copied, setCopied] = useState(false);
  function copy() {
    navigator.clipboard.writeText(value);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }
  return (
    <div className="connect-key-box" style={{ marginBottom: '0.75rem' }}>
      {label && <span style={{ color: '#718096', fontSize: '0.75rem', marginRight: '0.5rem', whiteSpace: 'nowrap' }}>{label}</span>}
      <code style={{ flex: 1, wordBreak: 'break-all', fontSize: '0.75rem', color: '#90cdf4' }}>{value}</code>
      <button className="connect-copy-btn" onClick={copy}>{copied ? '✓' : 'Copy'}</button>
    </div>
  );
}

// ── Platform setup guides shown after key creation ────────────────────────────

function PlatformGuides({ agentKey }) {
  const [tab, setTab] = useState('siri');

  const claudeMcpConfig = JSON.stringify({
    mcpServers: {
      hsip: {
        command: 'curl',
        args: ['-s', '-X', 'POST', `${BASE_URL}/v1/messages/sign`,
               '-H', `Authorization: Bearer ${agentKey}`,
               '-H', 'Content-Type: application/json',
               '-d', '{"content":"__MESSAGE__"}']
      }
    }
  }, null, 2);

  const systemPrompt =
`You are connected to HSIP, the user's personal privacy layer running at ${BASE_URL}.

To send a signed message:
POST ${BASE_URL}/v1/messages/sign
Authorization: Bearer ${agentKey}
{"content": "your message here"}

To read message history:
GET ${BASE_URL}/v1/messages
Authorization: Bearer ${agentKey}

When the user says things like "send HSIP message", "sign a message", or "record this", call the sign endpoint with their message text. Always confirm the message content before sending.`;

  const platforms = [
    { id: 'siri',   label: '🍎 Siri' },
    { id: 'claude', label: '🤖 Claude' },
    { id: 'api',    label: '⚡ Any AI' },
  ];

  return (
    <div style={{ marginTop: '1.25rem' }}>
      <p style={{ color: '#718096', fontSize: '0.8rem', marginBottom: '0.75rem' }}>
        Now set up your AI to use it:
      </p>
      <div className="os-picker" style={{ marginBottom: '1rem' }}>
        {platforms.map(p => (
          <button key={p.id}
            className={`os-btn${tab === p.id ? ' active' : ''}`}
            onClick={() => setTab(p.id)}>
            {p.label}
          </button>
        ))}
      </div>

      {tab === 'siri' && (
        <div className="integration-steps">
          <p className="connect-hint">
            Create a Siri Shortcut so you can say <em>"Hey Siri, send HSIP message"</em> and HSIP signs it instantly.
          </p>
          <div className="setup-step">
            <div className="setup-step-num">1</div>
            <div className="setup-step-body">
              <strong>Open the Shortcuts app</strong>
              <p>On iPhone/iPad: open <strong>Shortcuts</strong> → tap <strong>+</strong> to create a new shortcut.</p>
            </div>
          </div>
          <div className="setup-step">
            <div className="setup-step-num">2</div>
            <div className="setup-step-body">
              <strong>Add "Ask for Input" action</strong>
              <p>Search for <strong>Ask for Input</strong> → set prompt to <em>"What's your HSIP message?"</em> → save result as <code>MessageText</code>.</p>
            </div>
          </div>
          <div className="setup-step">
            <div className="setup-step-num">3</div>
            <div className="setup-step-body">
              <strong>Add "Get Contents of URL" action</strong>
              <p>Search for <strong>Get Contents of URL</strong> → set:</p>
              <CopyBox value={`${BASE_URL}/v1/messages/sign`} label="URL" />
              <p>Method: <strong>POST</strong> — Headers:</p>
              <CopyBox value={`Bearer ${agentKey}`} label="Authorization" />
              <CopyBox value="application/json" label="Content-Type" />
              <p>Request Body → JSON → add key <code>content</code> with value <code>MessageText</code> (tap the variable).</p>
            </div>
          </div>
          <div className="setup-step">
            <div className="setup-step-num">4</div>
            <div className="setup-step-body">
              <strong>Name it and add to Siri</strong>
              <p>Tap the shortcut name → rename to <strong>"Send HSIP Message"</strong> → tap <strong>Add to Siri</strong>. Now say <em>"Hey Siri, Send HSIP Message"</em>.</p>
            </div>
          </div>
        </div>
      )}

      {tab === 'claude' && (
        <div className="integration-steps">
          <p className="connect-hint">
            Connect Claude Desktop so it can send HSIP messages when you ask it to.
          </p>
          <div className="setup-step">
            <div className="setup-step-num">1</div>
            <div className="setup-step-body">
              <strong>Open Claude Desktop settings</strong>
              <p>Claude Desktop → top menu → <strong>Settings</strong> → <strong>Developer</strong> → <strong>Edit Config</strong>. This opens <code>claude_desktop_config.json</code>.</p>
            </div>
          </div>
          <div className="setup-step">
            <div className="setup-step-num">2</div>
            <div className="setup-step-body">
              <strong>Paste the system prompt in any conversation</strong>
              <p>Copy this and paste it at the start of a Claude conversation — it tells Claude what HSIP can do and how to call it:</p>
              <div className="connect-key-box" style={{ alignItems: 'flex-start', flexDirection: 'column', gap: '0.5rem' }}>
                <code style={{ fontSize: '0.7rem', color: '#90cdf4', whiteSpace: 'pre-wrap', wordBreak: 'break-word' }}>{systemPrompt}</code>
                <button className="connect-copy-btn" onClick={() => { navigator.clipboard.writeText(systemPrompt); }}>Copy</button>
              </div>
            </div>
          </div>
          <div className="setup-step">
            <div className="setup-step-num">3</div>
            <div className="setup-step-body">
              <strong>Ask Claude to send a message</strong>
              <p>Type: <em>"Send an HSIP message saying I confirmed the meeting at 3pm today"</em> — Claude will call the API and confirm.</p>
            </div>
          </div>
        </div>
      )}

      {tab === 'api' && (
        <div className="integration-steps">
          <p className="connect-hint">
            Any AI with HTTP request capability — ChatGPT Actions, custom agents, automations — can use HSIP directly.
          </p>
          <div className="setup-step">
            <div className="setup-step-num">1</div>
            <div className="setup-step-body">
              <strong>Inject these capabilities into your AI's system prompt</strong>
              <p>Open a browser tab to get the machine-readable capabilities spec — paste it into your AI's system prompt or context:</p>
              <CopyBox value={`${BASE_URL}/v1/agent/capabilities`} label="Capabilities URL" />
              <p style={{ fontSize: '0.8rem', color: '#718096' }}>Authorization: Bearer {agentKey.slice(0, 12)}…</p>
            </div>
          </div>
          <div className="setup-step">
            <div className="setup-step-num">2</div>
            <div className="setup-step-body">
              <strong>Send a message</strong>
              <div className="setup-code-block">
                <pre>{`POST ${BASE_URL}/v1/messages/sign
Authorization: Bearer ${agentKey}
Content-Type: application/json

{"content": "your message text"}`}</pre>
              </div>
            </div>
          </div>
          <div className="setup-step">
            <div className="setup-step-num">3</div>
            <div className="setup-step-body">
              <strong>Note for ChatGPT Actions</strong>
              <p>ChatGPT cannot reach <code>localhost</code> directly. To use HSIP with a Custom GPT, you need to expose HSIP to the internet first using a tool like <strong>ngrok</strong> (<code>ngrok http 7777</code>) — then use the ngrok URL instead of localhost.</p>
            </div>
          </div>
        </div>
      )}
    </div>
  );
}

// ── Connect AI dialog ─────────────────────────────────────────────────────────

function ConnectDialog({ apiKey, onDone, onClose }) {
  const [name,   setName]   = useState('');
  const [expiry, setExpiry] = useState('never');
  const [busy,   setBusy]   = useState(false);
  const [newKey, setNewKey] = useState(null);
  const [keyCopied, setKeyCopied] = useState(false);

  async function create() {
    if (!name.trim()) return;
    setBusy(true);
    try {
      const body = {
        name:            name.trim(),
        agent_type:      'ai_agent',
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
    setKeyCopied(true);
    setTimeout(() => setKeyCopied(false), 2000);
  }

  if (newKey) {
    return (
      <div className="connect-dialog">
        <div className="connect-dialog-inner" style={{ maxWidth: 560, maxHeight: '90vh', overflowY: 'auto' }}>
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

          <PlatformGuides agentKey={newKey} />

          <button className="primary" onClick={onClose}
            style={{ width: '100%', marginTop: '1.5rem' }}>
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
          Give this connection a name so you know which AI it belongs to —
          e.g. <em>"My Siri Shortcut"</em>, <em>"Claude Desktop"</em>, <em>"Home assistant"</em>.
        </p>
        <label className="connect-label">Name</label>
        <input
          className="connect-input"
          placeholder="e.g. My Siri Shortcut"
          value={name}
          onChange={e => setName(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && create()}
          autoFocus
        />
        <label className="connect-label">Key expires</label>
        <select className="connect-input" value={expiry} onChange={e => setExpiry(e.target.value)}>
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

// ── Agent card ────────────────────────────────────────────────────────────────

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
      <button className="danger" onClick={() => onRevoke(agent.key_id)}>Disconnect</button>
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
    const id = setInterval(loadAgents, 10_000);
    return () => clearInterval(id);
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
          Connect your AI assistants — Siri, Claude, ChatGPT, or any custom agent —
          so they can send signed messages, record consent, and act on your behalf through HSIP.
          You see everything they do and can disconnect any of them instantly.
        </p>
      </div>

      {/* What can AI agents do */}
      <div className="card" style={{ marginBottom: '1rem' }}>
        <h2>What connected AIs can do</h2>
        <div className="protection-grid">
          <div className="protection-item">
            <span>✉️</span>
            <div>
              <strong>Send signed messages</strong>
              <p>Say <em>"Send HSIP message: I agree to the terms"</em> — your AI signs it with your private key and timestamps it as legal proof.</p>
            </div>
          </div>
          <div className="protection-item">
            <span>📋</span>
            <div>
              <strong>Record consent</strong>
              <p>Ask your AI to log a consent decision and it creates a tamper-proof record with a cryptographic timestamp you can use in court.</p>
            </div>
          </div>
          <div className="protection-item">
            <span>🔍</span>
            <div>
              <strong>Verify messages</strong>
              <p>Paste a message you received and your AI checks whether the signature is genuine — confirming exactly who sent it.</p>
            </div>
          </div>
          <div className="protection-item">
            <span>🎙️</span>
            <div>
              <strong>Voice commands via Siri</strong>
              <p><em>"Hey Siri, send HSIP message"</em> — speak your message and it's signed and stored in seconds.</p>
            </div>
          </div>
        </div>
      </div>

      {loading && <div className="card"><p className="empty">Loading…</p></div>}

      {!loading && agents.length === 0 && (
        <div className="card">
          <div className="aiwatch-empty">
            <div className="aiwatch-empty-icon">🔌</div>
            <strong>No AI systems connected yet</strong>
            <p>Connect your first AI assistant to start sending messages by voice or through your favourite AI tools.</p>
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
            {anomalous.length} connection{anomalous.length !== 1 ? 's have' : ' has'} triggered
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
          <p className="aiwatch-normal-note">All activity looks normal. Disconnect any connection at any time.</p>
          {normal.map(a => (
            <AgentCard key={a.key_id} agent={a} onRevoke={revokeAgent} />
          ))}
        </div>
      )}
    </div>
  );
}
