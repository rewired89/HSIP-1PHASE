import React, { useState, useEffect } from 'react';
import { request } from '../api';

export default function Audit({ apiKey }) {
  const [entries, setEntries] = useState([]);
  const [filter,  setFilter]  = useState('');
  const [chain,   setChain]   = useState(null);
  const [checkingChain, setCheckingChain] = useState(true);

  useEffect(() => { load(); checkChain(); }, []);

  async function load() {
    const params = filter ? ('?action=' + filter) : '';
    try { setEntries(await request('GET', '/v1/audit' + params, null, apiKey)); } catch {}
  }

  async function checkChain() {
    setCheckingChain(true);
    try { setChain(await request('GET', '/v1/audit/verify', null, apiKey)); }
    catch { setChain(null); }
    setCheckingChain(false);
  }

  return (
    <div>
      <div className="card">
        <h2>Audit Log</h2>
        <p style={{ color: '#718096', marginBottom: '1rem' }}>
          Tamper-evident record of all operations. Suitable for compliance reporting and legal proceedings.
        </p>

        {!checkingChain && chain && (
          <div style={{ display: 'flex', alignItems: 'center', gap: '0.6rem', marginBottom: '1rem' }}>
            <span className={`badge ${chain.valid ? 'verified' : 'failed'}`}>
              {chain.valid ? '✓ Hash chain intact' : '✗ Hash chain broken'}
            </span>
            <span style={{ color: '#718096', fontSize: '0.78rem' }}>
              {chain.checked} entr{chain.checked === 1 ? 'y' : 'ies'} checked
              {chain.unchained > 0 && `, ${chain.unchained} pre-chain (unchained)`}
              {!chain.valid && chain.first_break_id && ` — first break at ${chain.first_break_id.slice(0, 8)}...`}
            </span>
            <button className="consumer-reset-btn" onClick={checkChain} style={{ marginLeft: 'auto' }}>
              Re-check
            </button>
          </div>
        )}
        {checkingChain && <p style={{ color: '#718096', fontSize: '0.8rem', marginBottom: '1rem' }}>Checking hash chain…</p>}

        <div style={{ display: 'flex', gap: '0.75rem', marginBottom: '1rem' }}>
          <input
            placeholder="Filter by action (e.g. consent, message)..."
            value={filter}
            onChange={e => setFilter(e.target.value)}
            style={{ flex: 1, marginBottom: 0 }}
          />
          <button className="primary" onClick={load}>Filter</button>
        </div>
        {entries.length === 0
          ? <p className="empty">No audit entries yet.</p>
          : (
            <table>
              <thead><tr><th>Action</th><th>Peer Key</th><th>Details</th><th>Timestamp</th></tr></thead>
              <tbody>
                {entries.map(e => (
                  <tr key={e.id}>
                    <td><code style={{ fontSize: '0.8rem' }}>{e.action}</code></td>
                    <td style={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>
                      {e.peer_verify_key ? e.peer_verify_key.slice(0, 12) + '...' : '—'}
                    </td>
                    <td style={{ fontSize: '0.8rem', color: '#718096' }}>{e.details || '—'}</td>
                    <td style={{ fontSize: '0.8rem' }}>{new Date(e.timestamp).toLocaleString()}</td>
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
