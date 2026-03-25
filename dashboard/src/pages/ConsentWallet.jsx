import React, { useState, useEffect } from 'react';
import { request } from '../api';

const DURATION_OPTIONS = [
  { label: '1 hour',   ms: 3_600_000 },
  { label: '24 hours', ms: 86_400_000 },
  { label: '1 week',   ms: 604_800_000 },
  { label: '30 days',  ms: 2_592_000_000 },
];

function timeUntil(dateStr) {
  const diff = new Date(dateStr) - Date.now();
  if (diff <= 0) return 'Expired';
  const mins  = Math.floor(diff / 60_000);
  const hours = Math.floor(mins  / 60);
  const days  = Math.floor(hours / 24);
  if (days  > 0) return `Expires in ${days} day${days  !== 1 ? 's' : ''}`;
  if (hours > 0) return `Expires in ${hours} hour${hours !== 1 ? 's' : ''}`;
  return `Expires in ${mins} minute${mins !== 1 ? 's' : ''}`;
}

export default function ConsentWallet({ apiKey }) {
  const [consents,   setConsents]   = useState([]);
  const [peerKey,    setPeerKey]    = useState('');
  const [duration,   setDuration]   = useState(DURATION_OPTIONS[0].ms);
  const [showGrant,  setShowGrant]  = useState(false);
  const [granting,   setGranting]   = useState(false);

  useEffect(() => { loadConsents(); }, []);

  async function loadConsents() {
    try { setConsents(await request('GET', '/v1/consent', null, apiKey)); } catch {}
  }

  async function grant() {
    if (!peerKey.trim()) return;
    setGranting(true);
    try {
      await request('POST', '/v1/consent/grant',
        { peer_verify_key: peerKey.trim(), ttl_ms: duration }, apiKey);
      setPeerKey('');
      setShowGrant(false);
      await loadConsents();
    } catch {
      alert('Could not give access. Make sure the HSIP ID is correct and try again.');
    }
    setGranting(false);
  }

  async function revoke(key) {
    if (!confirm('Remove this access? They will no longer be able to send you verified messages.')) return;
    try {
      await request('POST', '/v1/consent/revoke', { peer_verify_key: key }, apiKey);
      await loadConsents();
    } catch (e) { alert(e.message); }
  }

  const active = consents.filter(c => c.status === 'granted');
  const past   = consents.filter(c => c.status !== 'granted');

  return (
    <div>
      <div className="consumer-hero">
        <div className="consumer-hero-icon">🛡️</div>
        <h2>My Consent Wallet</h2>
        <p>
          You decide who can communicate with you. Every permission is cryptographically
          recorded and can be removed at any time — instantly.
        </p>
      </div>

      <div className="wallet-summary card">
        <div className="wallet-stat">
          <span className="wallet-stat-number">{active.length}</span>
          <span className="wallet-stat-label">Active permissions</span>
        </div>
        <div className="wallet-stat">
          <span className="wallet-stat-number">{past.length}</span>
          <span className="wallet-stat-label">Past / expired</span>
        </div>
        <button
          className="primary"
          style={{ marginLeft: 'auto' }}
          onClick={() => setShowGrant(v => !v)}
        >
          {showGrant ? '✕ Cancel' : '+ Give Access'}
        </button>
      </div>

      {showGrant && (
        <div className="card grant-panel">
          <h3>Give someone access</h3>
          <p className="grant-desc">
            Paste their HSIP ID below. During the time window you choose, they'll be able to
            send you cryptographically verified messages.
          </p>

          <div className="consumer-field-label">Their HSIP ID</div>
          <input
            placeholder="Paste their public ID here…"
            value={peerKey}
            onChange={e => setPeerKey(e.target.value)}
          />

          <div className="consumer-field-label" style={{ marginTop: '0.25rem' }}>How long?</div>
          <div className="duration-selector">
            {DURATION_OPTIONS.map(opt => (
              <button
                key={opt.ms}
                className={`dur-btn${duration === opt.ms ? ' active' : ''}`}
                onClick={() => setDuration(opt.ms)}
              >
                {opt.label}
              </button>
            ))}
          </div>

          <button
            className="primary consumer-action-btn"
            onClick={grant}
            disabled={!peerKey.trim() || granting}
            style={{ marginTop: '1rem' }}
          >
            {granting ? 'Giving access…' : 'Give Access'}
          </button>
        </div>
      )}

      <div className="card">
        <h2>Active Permissions</h2>
        {active.length === 0 ? (
          <p className="empty">No one has access right now.</p>
        ) : (
          <div className="consent-cards">
            {active.map(c => (
              <div key={c.id} className="consent-card">
                <div className="consent-card-left">
                  <div className="consent-avatar">👤</div>
                  <div>
                    <div className="consent-id">
                      ID: {c.peer_verify_key.slice(0, 12)}…{c.peer_verify_key.slice(-6)}
                    </div>
                    <div className="consent-expires">
                      {c.expires_at ? timeUntil(c.expires_at) : 'No expiry set'}
                    </div>
                  </div>
                </div>
                <button className="danger" onClick={() => revoke(c.peer_verify_key)}>
                  Remove Access
                </button>
              </div>
            ))}
          </div>
        )}
      </div>

      {past.length > 0 && (
        <div className="card">
          <h2>Past &amp; Expired</h2>
          <div className="consent-cards">
            {past.map(c => (
              <div key={c.id} className="consent-card consent-card-inactive">
                <div className="consent-card-left">
                  <div className="consent-avatar" style={{ opacity: 0.4 }}>👤</div>
                  <div>
                    <div className="consent-id">
                      ID: {c.peer_verify_key.slice(0, 12)}…{c.peer_verify_key.slice(-6)}
                    </div>
                    <div className="consent-expires">
                      <span className={`badge ${c.status}`}>{c.status}</span>
                      {c.expires_at && ` · ${new Date(c.expires_at).toLocaleDateString()}`}
                    </div>
                  </div>
                </div>
              </div>
            ))}
          </div>
        </div>
      )}

      <div className="consumer-explainer card">
        <h3>Why does this matter?</h3>
        <p className="explainer-body">
          Unlike a checkbox in a privacy policy, every "access granted" and "access removed"
          event here is cryptographically signed and stored in a tamper-proof audit log.
          These records can't be altered after the fact — giving you genuine, verifiable proof
          of what you allowed and exactly when.
        </p>
      </div>
    </div>
  );
}
