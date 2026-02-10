import React, { useState, useEffect } from 'react';
import { request } from '../api';

export default function Audit({ apiKey }) {
  const [entries, setEntries] = useState([]);
  const [filter,  setFilter]  = useState('');

  useEffect(() => { load(); }, []);

  async function load() {
    const params = filter ? ('?action=' + filter) : '';
    try { setEntries(await request('GET', '/v1/audit' + params, null, apiKey)); } catch {}
  }

  return (
    <div>
      <div className="card">
        <h2>Audit Log</h2>
        <p style={{ color: '#718096', marginBottom: '1rem' }}>
          Tamper-evident record of all operations. Suitable for compliance reporting and legal proceedings.
        </p>
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
