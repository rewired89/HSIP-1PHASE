import React, { useState, useEffect } from 'react';
import { request } from '../api';

export default function Consent({ apiKey }) {
  const [consents, setConsents] = useState([]);
  const [peerKey,  setPeerKey]  = useState('');
  const [ttl,      setTtl]      = useState('3600000');

  useEffect(() => { loadConsents(); }, []);

  async function loadConsents() {
    try { setConsents(await request('GET', '/v1/consent', null, apiKey)); } catch {}
  }

  async function grant() {
    try {
      await request('POST', '/v1/consent/grant', { peer_verify_key: peerKey, ttl_ms: parseInt(ttl) }, apiKey);
      setPeerKey('');
      loadConsents();
    } catch (e) { alert(e.message); }
  }

  async function revoke(key) {
    try {
      await request('POST', '/v1/consent/revoke', { peer_verify_key: key }, apiKey);
      loadConsents();
    } catch (e) { alert(e.message); }
  }

  return (
    <div>
      <div className="card">
        <h2>Grant Consent</h2>
        <input placeholder="Peer verify key (base64)" value={peerKey} onChange={e => setPeerKey(e.target.value)} />
        <input placeholder="TTL in ms (default: 3600000 = 1 hour)" value={ttl} onChange={e => setTtl(e.target.value)} />
        <button className="primary" onClick={grant}>Grant Consent</button>
      </div>
      <div className="card">
        <h2>Active Consents</h2>
        {consents.length === 0
          ? <p className="empty">No consents yet.</p>
          : (
            <table>
              <thead><tr><th>Peer Key</th><th>Status</th><th>Expires</th><th>Action</th></tr></thead>
              <tbody>
                {consents.map(c => (
                  <tr key={c.id}>
                    <td style={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>{c.peer_verify_key.slice(0, 16)}...</td>
                    <td><span className={'badge ' + c.status}>{c.status}</span></td>
                    <td>{c.expires_at ? new Date(c.expires_at).toLocaleString() : '—'}</td>
                    <td>
                      {c.status === 'granted' && (
                        <button className="danger" onClick={() => revoke(c.peer_verify_key)}>Revoke</button>
                      )}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )
        }
      </div>
    </div>
  );
}
