import React, { useState, useEffect } from 'react';
import { request } from '../api';

export default function Keys({ apiKey }) {
  const [keys,    setKeys]   = useState([]);
  const [newName, setNewName] = useState('');
  const [newKey,  setNewKey]  = useState(null);

  useEffect(() => { loadKeys(); }, []);

  async function loadKeys() {
    try { setKeys(await request('GET', '/v1/keys', null, apiKey)); } catch {}
  }

  async function create() {
    try {
      const r = await request('POST', '/v1/keys', { name: newName || 'default' }, apiKey);
      setNewKey(r.key);
      setNewName('');
      loadKeys();
    } catch (e) { alert(e.message); }
  }

  async function revoke(id) {
    if (!window.confirm('Revoke this key? This cannot be undone.')) return;
    try { await request('DELETE', '/v1/keys/' + id, null, apiKey); loadKeys(); }
    catch (e) { alert(e.message); }
  }

  return (
    <div>
      {newKey && (
        <div className="card" style={{ borderColor: '#68d391' }}>
          <h2>New API Key — Save this now!</h2>
          <div className="key-display">{newKey}</div>
          <p style={{ color: '#718096', marginTop: '0.75rem', fontSize: '0.8rem' }}>
            This key will not be shown again.
          </p>
          <button className="primary" style={{ marginTop: '0.75rem' }} onClick={() => setNewKey(null)}>
            I have saved it
          </button>
        </div>
      )}
      <div className="card">
        <h2>Create API Key</h2>
        <input placeholder="Key name (optional)" value={newName} onChange={e => setNewName(e.target.value)} />
        <button className="primary" onClick={create}>Create Key</button>
      </div>
      <div className="card">
        <h2>API Keys</h2>
        {keys.length === 0
          ? <p className="empty">No keys found.</p>
          : (
            <table>
              <thead><tr><th>ID</th><th>Name</th><th>Status</th><th>Created</th><th>Action</th></tr></thead>
              <tbody>
                {keys.map(k => (
                  <tr key={k.id}>
                    <td style={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>{k.id.slice(0, 8)}...</td>
                    <td>{k.name}</td>
                    <td><span className={'badge ' + (k.active ? 'granted' : 'revoked')}>{k.active ? 'active' : 'revoked'}</span></td>
                    <td style={{ fontSize: '0.8rem' }}>{new Date(k.created_at).toLocaleString()}</td>
                    <td>{k.active && <button className="danger" onClick={() => revoke(k.id)}>Revoke</button>}</td>
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
