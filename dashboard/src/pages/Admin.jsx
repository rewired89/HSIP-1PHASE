import React, { useState, useEffect } from 'react';
import { request } from '../api';

function MasterKeyCard({ apiKey }) {
  const [info, setInfo] = useState(null);
  const [error, setError] = useState(null);
  const [confirming, setConfirming] = useState(false);
  const [rotating, setRotating] = useState(false);
  const [result, setResult] = useState(null);

  useEffect(() => { load(); }, []);

  async function load() {
    setError(null);
    try { setInfo(await request('GET', '/v1/admin/master-key/fingerprint', null, apiKey)); }
    catch (e) { setError(e.message); }
  }

  async function rotate() {
    setRotating(true);
    try {
      const r = await request('POST', '/v1/admin/master-key/rotate', null, apiKey);
      setResult(r);
      setConfirming(false);
      load();
    } catch (e) { alert(e.message); }
    setRotating(false);
  }

  return (
    <div className="card">
      <h2>Master Encryption Key</h2>
      <p style={{ color: '#718096', marginBottom: '1rem' }}>
        Every tenant's Ed25519 signing key is encrypted at rest under this key. Rotating
        it re-encrypts every identity under a brand-new key — the old key stops working
        the moment rotation completes. Root-admin only.
      </p>

      {error && (
        <div style={{ padding: '0.85rem', borderRadius: '6px', background: '#2d1b1b', border: '1px solid #fc8181' }}>
          <p style={{ color: '#fc8181', fontSize: '0.85rem' }}>
            {error.includes('root') || error.includes('admin') || error.includes('Unauthorized')
              ? 'Your key is not a root admin, so master key operations aren’t available to it.'
              : error}
          </p>
        </div>
      )}

      {info && !error && (
        <>
          <table>
            <tbody>
              <tr><td style={{ color: '#718096' }}>Fingerprint</td><td style={{ fontFamily: 'monospace' }}>{info.fingerprint}</td></tr>
              <tr><td style={{ color: '#718096' }}>Source</td><td>{info.master_key_path || 'HSIP_MASTER_KEY env var'}</td></tr>
              <tr>
                <td style={{ color: '#718096' }}>Rotation</td>
                <td>
                  <span className={`badge ${info.rotation_available ? 'granted' : 'revoked'}`}>
                    {info.rotation_available ? 'available' : 'not configured'}
                  </span>
                </td>
              </tr>
            </tbody>
          </table>

          {!confirming ? (
            <button
              className="danger"
              style={{ marginTop: '1rem' }}
              onClick={() => setConfirming(true)}
              disabled={!info.rotation_available}
            >
              Rotate master key
            </button>
          ) : (
            <div style={{ marginTop: '1rem', padding: '1rem', borderRadius: '6px', background: '#2d1b1b', border: '1px solid #fc8181' }}>
              <p style={{ color: '#fc8181', fontWeight: 'bold', marginBottom: '0.5rem' }}>
                Are you sure? Every identity's signing key will be re-encrypted immediately.
              </p>
              <div style={{ display: 'flex', gap: '0.5rem' }}>
                <button className="consumer-reset-btn" onClick={() => setConfirming(false)}>Cancel</button>
                <button className="danger" onClick={rotate} disabled={rotating}>
                  {rotating ? 'Rotating…' : 'Yes, rotate now'}
                </button>
              </div>
            </div>
          )}

          {result && (
            <div style={{ marginTop: '1rem', padding: '1rem', borderRadius: '6px', background: '#1a2d1a', border: '1px solid #68d391' }}>
              <p style={{ color: '#68d391', fontWeight: 'bold' }}>✓ Rotated</p>
              <p style={{ fontSize: '0.8rem', color: '#a0aec0' }}>
                {result.identities_reencrypted} identit{result.identities_reencrypted === 1 ? 'y' : 'ies'} re-encrypted
                {result.anchor_identity_reencrypted ? ', plus the node anchor identity' : ''}.
              </p>
              <p style={{ fontSize: '0.75rem', color: '#a0aec0', fontFamily: 'monospace', marginTop: '0.5rem' }}>
                {result.old_key_fingerprint} → {result.new_key_fingerprint}
              </p>
            </div>
          )}
        </>
      )}
    </div>
  );
}

function RootAdminsCard({ apiKey }) {
  const [admins, setAdmins] = useState([]);
  const [error,  setError]  = useState(null);
  const [keyId,  setKeyId]  = useState('');
  const [busy,   setBusy]   = useState(false);

  useEffect(() => { load(); }, []);

  async function load() {
    setError(null);
    try { setAdmins(await request('GET', '/v1/admin/root-admins', null, apiKey)); }
    catch (e) { setError(e.message); }
  }

  async function grant() {
    if (!keyId.trim()) return;
    setBusy(true);
    try {
      await request('POST', '/v1/admin/root-admins/grant', { key_id: keyId.trim() }, apiKey);
      setKeyId('');
      load();
    } catch (e) { alert(e.message); }
    setBusy(false);
  }

  async function revoke(id) {
    if (!window.confirm('Revoke root-admin from this key?')) return;
    try { await request('POST', '/v1/admin/root-admins/revoke', { key_id: id }, apiKey); load(); }
    catch (e) { alert(e.message); }
  }

  if (error) return null; // MasterKeyCard above already surfaces the "not a root admin" message once.

  return (
    <div className="card">
      <h2>Root Admins</h2>
      <p style={{ color: '#718096', marginBottom: '1rem' }}>
        Node-level authority (master key rotation, granting/revoking other root admins).
        Separate from a tenant's <code>owner</code> role, which only manages keys within
        one tenant.
      </p>

      <div style={{ display: 'flex', gap: '0.75rem', marginBottom: '1rem' }}>
        <input
          placeholder="Key ID to grant root-admin"
          value={keyId}
          onChange={e => setKeyId(e.target.value)}
          style={{ flex: 1, marginBottom: 0, fontFamily: 'monospace', fontSize: '0.85rem' }}
        />
        <button className="primary" onClick={grant} disabled={busy || !keyId.trim()}>Grant</button>
      </div>

      {admins.length === 0
        ? <p className="empty">No root admins found.</p>
        : (
          <table>
            <thead><tr><th>ID</th><th>Name</th><th>Tenant</th><th>Since</th><th></th></tr></thead>
            <tbody>
              {admins.map(a => (
                <tr key={a.id}>
                  <td style={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>{a.id.slice(0, 8)}...</td>
                  <td>{a.name}</td>
                  <td style={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>{a.tenant_id.slice(0, 8)}...</td>
                  <td style={{ fontSize: '0.8rem' }}>{new Date(a.created_at).toLocaleString()}</td>
                  <td>
                    <button className="danger" onClick={() => revoke(a.id)} disabled={admins.length <= 1}>
                      Revoke
                    </button>
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )
      }
    </div>
  );
}

export default function Admin({ apiKey }) {
  return (
    <div>
      <MasterKeyCard apiKey={apiKey} />
      <RootAdminsCard apiKey={apiKey} />
    </div>
  );
}
