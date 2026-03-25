import React, { useState, useEffect } from 'react';
import { request } from '../api';

export default function ProveIt({ apiKey }) {
  const [tab,         setTab]        = useState('create');
  const [message,     setMessage]    = useState('');
  const [certificate, setCertificate] = useState(null);
  const [copied,      setCopied]     = useState(false);
  const [pastedCert,  setPastedCert] = useState('');
  const [verifyResult, setVerifyResult] = useState(null);
  const [loading,     setLoading]    = useState(false);
  const [identity,    setIdentity]   = useState(null);

  useEffect(() => {
    request('GET', '/v1/identity', null, apiKey)
      .then(id => setIdentity(id))
      .catch(() => {});
  }, []);

  async function createProof() {
    if (!message.trim()) return;
    setLoading(true);
    try {
      const id = identity || await request('POST', '/v1/identity', null, apiKey);
      if (!identity) setIdentity(id);
      const signed = await request('POST', '/v1/messages/sign', { content: message }, apiKey);
      setCertificate({
        hsip_proof: true,
        version: '1.0',
        message,
        signed_at: new Date().toISOString(),
        signed_by: id.verify_key,
        signature: signed.signature,
      });
    } catch {
      alert('Something went wrong. Please try again.');
    }
    setLoading(false);
  }

  async function checkProof() {
    if (!pastedCert.trim()) return;
    setLoading(true);
    try {
      const cert = JSON.parse(pastedCert);
      if (!cert.hsip_proof || !cert.signature || !cert.signed_by || !cert.message) {
        throw new Error('This does not look like a valid HSIP proof certificate.');
      }
      const result = await request('POST', '/v1/messages/verify', {
        content: cert.message,
        signature: cert.signature,
        peer_verify_key: cert.signed_by,
      }, apiKey);
      setVerifyResult({ valid: result.verified, cert });
    } catch (e) {
      setVerifyResult({ valid: false, error: e.message });
    }
    setLoading(false);
  }

  function copyCertificate() {
    navigator.clipboard.writeText(JSON.stringify(certificate, null, 2));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  function resetCreate() {
    setCertificate(null);
    setMessage('');
  }

  function resetVerify() {
    setVerifyResult(null);
    setPastedCert('');
  }

  return (
    <div>
      <div className="consumer-hero">
        <div className="consumer-hero-icon">✍️</div>
        <h2>Prove It</h2>
        <p>
          Sign any message, agreement, or creative work so anyone can verify it genuinely came from
          you — and hasn't been changed since.
        </p>
      </div>

      <div className="consumer-tabs">
        <button
          className={tab === 'create' ? 'active' : ''}
          onClick={() => { setTab('create'); resetVerify(); }}
        >
          Create a Proof
        </button>
        <button
          className={tab === 'verify' ? 'active' : ''}
          onClick={() => { setTab('verify'); resetCreate(); }}
        >
          Check a Proof
        </button>
      </div>

      {tab === 'create' && (
        <div className="card">
          {!certificate ? (
            <>
              <div className="consumer-field-label">What do you want to prove?</div>
              <div className="consumer-field-hint">
                Type a message, agreement, creative work, or any statement you want timestamped proof of.
              </div>
              <textarea
                rows={5}
                placeholder="Example: I, Jane Smith, agreed to design 3 logos for $500 on this date. Scope: homepage, about page, logo."
                value={message}
                onChange={e => setMessage(e.target.value)}
                style={{ marginTop: '0.75rem' }}
              />
              <button
                className="primary consumer-action-btn"
                onClick={createProof}
                disabled={!message.trim() || loading}
              >
                {loading ? 'Signing…' : '🔒 Sign & Create Proof'}
              </button>
            </>
          ) : (
            <div className="proof-result">
              <div className="proof-result-header">
                <span className="proof-check">✅</span>
                <div>
                  <strong>Proof Created</strong>
                  <div className="proof-meta">
                    Signed on {new Date(certificate.signed_at).toLocaleString()}
                  </div>
                </div>
              </div>

              <div className="proof-message-preview">"{certificate.message}"</div>

              <div className="consumer-field-label" style={{ marginTop: '1.25rem' }}>
                Your Proof Certificate
              </div>
              <div className="consumer-field-hint">
                Copy this and share it with whoever needs to verify your message is authentic.
                Anyone with an HSIP account can paste it into "Check a Proof".
              </div>
              <div className="key-display cert-display">
                {JSON.stringify(certificate, null, 2)}
              </div>

              <div style={{ display: 'flex', gap: '0.75rem', marginTop: '0.75rem', flexWrap: 'wrap' }}>
                <button className="primary" onClick={copyCertificate}>
                  {copied ? '✓ Copied!' : '📋 Copy Certificate'}
                </button>
                <button className="consumer-reset-btn" onClick={resetCreate}>
                  Create another proof
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {tab === 'verify' && (
        <div className="card">
          {!verifyResult ? (
            <>
              <div className="consumer-field-label">Paste a proof certificate</div>
              <div className="consumer-field-hint">
                The person who created the proof should have shared a certificate code with you.
                Paste it below to check if it's genuine.
              </div>
              <textarea
                rows={10}
                placeholder={'Paste the certificate here…\n(starts with {\n  "hsip_proof": true,\n  ...)\n}'}
                value={pastedCert}
                onChange={e => setPastedCert(e.target.value)}
                style={{ marginTop: '0.75rem', fontFamily: 'monospace', fontSize: '0.8rem' }}
              />
              <button
                className="primary consumer-action-btn"
                onClick={checkProof}
                disabled={!pastedCert.trim() || loading}
              >
                {loading ? 'Checking…' : '🔍 Check This Proof'}
              </button>
            </>
          ) : (
            <div className="proof-result">
              {verifyResult.valid ? (
                <>
                  <div className="proof-result-header">
                    <span className="proof-check">✅</span>
                    <div>
                      <strong style={{ color: '#68d391' }}>Proof is Genuine</strong>
                      <div className="proof-meta">
                        This message was cryptographically signed and has not been altered.
                      </div>
                    </div>
                  </div>
                  <div className="consumer-field-label" style={{ marginTop: '1.25rem' }}>
                    The verified message:
                  </div>
                  <div className="proof-message-preview">"{verifyResult.cert.message}"</div>
                  <div className="proof-meta" style={{ marginTop: '0.75rem' }}>
                    Originally signed on{' '}
                    {new Date(verifyResult.cert.signed_at).toLocaleString()}
                  </div>
                </>
              ) : (
                <div className="proof-result-header">
                  <span className="proof-check">❌</span>
                  <div>
                    <strong style={{ color: '#fc8181' }}>Could Not Verify</strong>
                    <div className="proof-meta">
                      {verifyResult.error ||
                        'The certificate may be altered, corrupted, or from an unknown source.'}
                    </div>
                  </div>
                </div>
              )}
              <button className="consumer-reset-btn" style={{ marginTop: '1.25rem' }} onClick={resetVerify}>
                Check another proof
              </button>
            </div>
          )}
        </div>
      )}

      <div className="consumer-explainer card">
        <h3>How does this work?</h3>
        <div className="explainer-steps">
          <div className="explainer-step">
            <span>1</span>
            <p>
              You type a message and click "Sign". HSIP uses your private cryptographic key
              to create a unique fingerprint of your exact words.
            </p>
          </div>
          <div className="explainer-step">
            <span>2</span>
            <p>
              You receive a "proof certificate" — a small bundle containing your message,
              a timestamp, and an unforgeable signature only your key could produce.
            </p>
          </div>
          <div className="explainer-step">
            <span>3</span>
            <p>
              Anyone can paste that certificate into "Check a Proof" to confirm it came from
              you and hasn't been modified — even a single character change will fail the check.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
