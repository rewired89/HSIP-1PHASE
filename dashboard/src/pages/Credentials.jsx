import React, { useState, useEffect } from 'react';
import { request } from '../api';

export default function Credentials({ apiKey }) {
  const [creds,      setCreds]     = useState([]);
  const [claim,      setClaim]     = useState('age_over_18');
  const [userToken,  setUserToken] = useState('');
  const [ttl,        setTtl]       = useState(86400);
  const [issued,     setIssued]    = useState(null);
  const [verifyJson, setVerifyJson] = useState('');
  const [verifyResult, setVerifyResult] = useState(null);

  useEffect(() => { loadCreds(); }, []);

  async function loadCreds() {
    try { setCreds(await request('GET', '/v1/credentials', null, apiKey)); } catch {}
  }

  async function issue() {
    if (!userToken.trim()) { alert('User token is required'); return; }
    try {
      const r = await request('POST', '/v1/credentials/issue', {
        claim,
        user_token: userToken,
        ttl_seconds: Number(ttl),
      }, apiKey);
      setIssued(r);
      setUserToken('');
      loadCreds();
    } catch (e) { alert(e.message); }
  }

  async function revoke(id) {
    if (!window.confirm('Revoke this credential? It will fail all future verifications.')) return;
    try {
      await request('DELETE', `/v1/credentials/${id}/revoke`, null, apiKey);
      loadCreds();
      if (issued?.credential?.id === id) setIssued(null);
    } catch (e) { alert(e.message); }
  }

  async function verify() {
    try {
      const parsed = JSON.parse(verifyJson);
      const r = await request('POST', '/v1/credentials/verify', parsed, apiKey);
      setVerifyResult(r);
    } catch (e) { setVerifyResult({ error: e.message }); }
  }

  return (
    <div>
      {/* Issue */}
      <div className="card">
        <h2>Issue Credential</h2>
        <p style={{ color: '#718096', fontSize: '0.85rem', marginBottom: '1rem' }}>
          Sign a claim on behalf of a subject. The subject presents this to any verifier.
        </p>
        <select value={claim} onChange={e => setClaim(e.target.value)}
          style={{ width: '100%', padding: '0.6rem', marginBottom: '0.75rem',
                   background: '#2d3748', color: '#e2e8f0', border: '1px solid #4a5568', borderRadius: '6px' }}>
          <option value="age_over_18">age_over_18</option>
          <option value="age_over_21">age_over_21</option>
          <option value="kyc_verified">kyc_verified</option>
          <option value="iso_27001">iso_27001</option>
          <option value="hipaa_compliant">hipaa_compliant</option>
          <option value="background_check_passed">background_check_passed</option>
          <option value="custom">custom</option>
        </select>
        {claim === 'custom' && (
          <input placeholder="Custom claim name" value={claim === 'custom' ? '' : claim}
            onChange={e => setClaim(e.target.value)} style={{ marginBottom: '0.75rem' }} />
        )}
        <input
          placeholder="User token (opaque ID — never the real identity)"
          value={userToken}
          onChange={e => setUserToken(e.target.value)}
        />
        <div style={{ display: 'flex', gap: '0.75rem', alignItems: 'center', marginTop: '0.75rem' }}>
          <select value={ttl} onChange={e => setTtl(e.target.value)}
            style={{ padding: '0.6rem', background: '#2d3748', color: '#e2e8f0',
                     border: '1px solid #4a5568', borderRadius: '6px' }}>
            <option value={3600}>Expires in 1 hour</option>
            <option value={86400}>Expires in 24 hours</option>
            <option value={604800}>Expires in 7 days</option>
            <option value={2592000}>Expires in 30 days</option>
            <option value={31536000}>Expires in 1 year</option>
          </select>
          <button className="primary" onClick={issue}>Issue Credential</button>
        </div>
      </div>

      {/* Issued result */}
      {issued && (
        <div className="card" style={{ borderColor: '#68d391' }}>
          <h2>Credential Issued — Share with the subject</h2>
          <p style={{ color: '#718096', fontSize: '0.8rem', marginBottom: '0.5rem' }}>
            This is the full credential packet the subject presents to verifiers.
          </p>
          <pre style={{ background: '#1a202c', padding: '1rem', borderRadius: '6px',
                        fontSize: '0.75rem', overflowX: 'auto', color: '#68d391' }}>
            {JSON.stringify(issued, null, 2)}
          </pre>
          <button className="primary" style={{ marginTop: '0.75rem' }}
            onClick={() => navigator.clipboard.writeText(JSON.stringify(issued))}>
            Copy to Clipboard
          </button>
        </div>
      )}

      {/* Verify */}
      <div className="card">
        <h2>Verify a Credential</h2>
        <p style={{ color: '#718096', fontSize: '0.85rem', marginBottom: '0.75rem' }}>
          Paste a credential packet to verify its signature, expiry, and revocation status.
        </p>
        <textarea
          placeholder='Paste credential JSON here: {"credential": {...}, "signature": "..."}'
          value={verifyJson}
          onChange={e => setVerifyJson(e.target.value)}
          rows={6}
          style={{ width: '100%', padding: '0.75rem', background: '#2d3748', color: '#e2e8f0',
                   border: '1px solid #4a5568', borderRadius: '6px', fontFamily: 'monospace',
                   fontSize: '0.8rem', resize: 'vertical', boxSizing: 'border-box' }}
        />
        <button className="primary" style={{ marginTop: '0.75rem' }} onClick={verify}>
          Verify Credential
        </button>
        {verifyResult && (
          <div style={{ marginTop: '1rem', padding: '1rem', borderRadius: '6px',
                        background: verifyResult.error ? '#2d1b1b' : verifyResult.valid ? '#1a2d1a' : '#2d1b1b',
                        border: `1px solid ${verifyResult.valid ? '#68d391' : '#fc8181'}` }}>
            {verifyResult.error ? (
              <p style={{ color: '#fc8181' }}>Error: {verifyResult.error}</p>
            ) : (
              <>
                <p style={{ color: verifyResult.valid ? '#68d391' : '#fc8181', fontWeight: 'bold', fontSize: '1.1rem' }}>
                  {verifyResult.valid ? '✓ VALID' : '✗ INVALID'}
                </p>
                <p style={{ color: '#a0aec0', fontSize: '0.85rem' }}>Claim: <b>{verifyResult.claim}</b></p>
                {verifyResult.expired && <p style={{ color: '#fc8181', fontSize: '0.85rem' }}>Reason: Expired</p>}
                {verifyResult.revoked && <p style={{ color: '#fc8181', fontSize: '0.85rem' }}>Reason: Revoked</p>}
                <p style={{ color: '#718096', fontSize: '0.8rem' }}>
                  Expires: {new Date(verifyResult.expires_at).toLocaleString()}
                </p>
              </>
            )}
          </div>
        )}
      </div>

      {/* List */}
      <div className="card">
        <h2>Issued Credentials</h2>
        {creds.length === 0
          ? <p className="empty">No credentials issued yet.</p>
          : (
            <table>
              <thead>
                <tr>
                  <th>Claim</th>
                  <th>User Token</th>
                  <th>Status</th>
                  <th>Issued</th>
                  <th>Expires</th>
                  <th>Action</th>
                </tr>
              </thead>
              <tbody>
                {creds.map(c => {
                  const expired = Date.now() > c.expires_at;
                  const status  = c.revoked ? 'revoked' : expired ? 'expired' : 'active';
                  return (
                    <tr key={c.id}>
                      <td><code style={{ fontSize: '0.8rem' }}>{c.claim}</code></td>
                      <td style={{ fontFamily: 'monospace', fontSize: '0.75rem' }}>
                        {c.user_token.slice(0, 16)}...
                      </td>
                      <td><span className={`badge ${status === 'active' ? 'granted' : 'revoked'}`}>{status}</span></td>
                      <td style={{ fontSize: '0.8rem' }}>{new Date(c.issued_at).toLocaleString()}</td>
                      <td style={{ fontSize: '0.8rem' }}>{new Date(c.expires_at).toLocaleString()}</td>
                      <td>
                        {!c.revoked && !expired && (
                          <button className="danger" onClick={() => revoke(c.id)}>Revoke</button>
                        )}
                      </td>
                    </tr>
                  );
                })}
              </tbody>
            </table>
          )
        }
      </div>
    </div>
  );
}
