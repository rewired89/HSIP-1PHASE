import React, { useState, useEffect } from 'react';
import { request } from '../api';

export default function Trust({ apiKey }) {
  const [peers,   setPeers]   = useState([]);
  const [label,   setLabel]   = useState('');
  const [verifyKey, setVerifyKey] = useState('');
  const [adding,  setAdding]  = useState(false);

  const [vLabel,   setVLabel]   = useState('');
  const [vContent, setVContent] = useState('');
  const [vSig,     setVSig]     = useState('');
  const [verifying, setVerifying] = useState(false);
  const [verifyResult, setVerifyResult] = useState(null);

  useEffect(() => { loadPeers(); }, []);

  async function loadPeers() {
    try { setPeers(await request('GET', '/v1/trust/peers', null, apiKey)); } catch {}
  }

  async function addPeer() {
    if (!label.trim() || !verifyKey.trim()) return;
    setAdding(true);
    try {
      await request('POST', '/v1/trust/peer', { label: label.trim(), verify_key: verifyKey.trim() }, apiKey);
      setLabel('');
      setVerifyKey('');
      loadPeers();
    } catch (e) { alert(e.message); }
    setAdding(false);
  }

  async function removePeer(id) {
    if (!window.confirm('Remove this trusted peer?')) return;
    try { await request('DELETE', '/v1/trust/peers/' + id, null, apiKey); loadPeers(); }
    catch (e) { alert(e.message); }
  }

  async function verifySignature() {
    if (!vLabel.trim() || !vContent.trim() || !vSig.trim()) return;
    setVerifying(true);
    setVerifyResult(null);
    try {
      const r = await request('POST', '/v1/trust/verify', {
        label: vLabel.trim(), content: vContent, signature: vSig.trim(),
      }, apiKey);
      setVerifyResult(r);
    } catch (e) { setVerifyResult({ error: e.message }); }
    setVerifying(false);
  }

  return (
    <div>
      <div className="card">
        <h2>Federated Trust</h2>
        <p style={{ color: '#718096', marginBottom: '1rem' }}>
          Store other HSIP nodes' or peers' Ed25519 verify keys under a human-readable
          label, so messages they signed can be verified locally — no shared secret,
          no live connection to them required.
        </p>
      </div>

      <div className="card">
        <h2>Add a trusted peer</h2>
        <input
          placeholder="Label (e.g. alice, partner-node)"
          value={label}
          onChange={e => setLabel(e.target.value)}
        />
        <input
          placeholder="Ed25519 verify key (base64)"
          value={verifyKey}
          onChange={e => setVerifyKey(e.target.value)}
          style={{ fontFamily: 'monospace', fontSize: '0.85rem' }}
        />
        <button className="primary" onClick={addPeer} disabled={adding || !label.trim() || !verifyKey.trim()}>
          {adding ? 'Adding…' : 'Add peer'}
        </button>
      </div>

      <div className="card">
        <h2>Trusted peers</h2>
        {peers.length === 0
          ? <p className="empty">No trusted peers yet. Add one above.</p>
          : (
            <table>
              <thead><tr><th>Label</th><th>Verify key</th><th>Added</th><th></th></tr></thead>
              <tbody>
                {peers.map(p => (
                  <tr key={p.id}>
                    <td>{p.label}</td>
                    <td style={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>
                      {p.verify_key.slice(0, 16)}...
                    </td>
                    <td style={{ fontSize: '0.8rem' }}>{new Date(p.added_at).toLocaleString()}</td>
                    <td><button className="danger" onClick={() => removePeer(p.id)}>Remove</button></td>
                  </tr>
                ))}
              </tbody>
            </table>
          )
        }
      </div>

      <div className="card">
        <h2>Verify a signature from a trusted peer</h2>
        <p style={{ color: '#718096', fontSize: '0.85rem', marginBottom: '0.75rem' }}>
          Check a message signature against a peer's stored verify key, by label —
          no need to paste the raw key again.
        </p>
        <input placeholder="Peer label" value={vLabel} onChange={e => setVLabel(e.target.value)} />
        <textarea
          placeholder="Message content"
          value={vContent}
          onChange={e => setVContent(e.target.value)}
          rows={3}
          style={{ width: '100%', marginBottom: '0.75rem', fontFamily: 'inherit' }}
        />
        <input
          placeholder="Signature (base64)"
          value={vSig}
          onChange={e => setVSig(e.target.value)}
          style={{ fontFamily: 'monospace', fontSize: '0.85rem' }}
        />
        <button className="primary" onClick={verifySignature} disabled={verifying}>
          {verifying ? 'Verifying…' : 'Verify'}
        </button>

        {verifyResult && (
          <div style={{ marginTop: '1rem', padding: '1rem', borderRadius: '6px',
                        background: verifyResult.error ? '#2d1b1b' : verifyResult.verified ? '#1a2d1a' : '#2d1b1b',
                        border: `1px solid ${verifyResult.error ? '#fc8181' : verifyResult.verified ? '#68d391' : '#fc8181'}` }}>
            {verifyResult.error ? (
              <p style={{ color: '#fc8181' }}>Error: {verifyResult.error}</p>
            ) : (
              <p style={{ color: verifyResult.verified ? '#68d391' : '#fc8181', fontWeight: 'bold' }}>
                {verifyResult.verified ? `✓ Valid signature from "${verifyResult.label}"` : '✗ Signature does not match'}
              </p>
            )}
          </div>
        )}
      </div>
    </div>
  );
}
