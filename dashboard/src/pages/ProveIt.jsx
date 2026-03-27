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
        <h2>Digital Alibi</h2>
        <p>
          Prove you sent that message. Prove no one tampered with your agreement.
          Sign anything — a contract, a statement, a creative work — and get a
          certificate that holds up even in a legal dispute.
          One changed word and the proof breaks. That's the point.
        </p>
      </div>

      <div className="consumer-tabs">
        <button
          className={tab === 'create' ? 'active' : ''}
          onClick={() => { setTab('create'); resetVerify(); }}
        >
          Create Proof
        </button>
        <button
          className={tab === 'verify' ? 'active' : ''}
          onClick={() => { setTab('verify'); resetCreate(); }}
        >
          Verify a Proof
        </button>
      </div>

      {tab === 'create' && (
        <div className="card">
          {!certificate ? (
            <>
              <div className="consumer-field-label">What do you need to prove?</div>
              <div className="consumer-field-hint">
                Type your message, agreement, or statement. Once signed, nobody can claim you said something different.
              </div>
              <textarea
                rows={5}
                placeholder="Example: I, Jane Smith, agreed to design 3 logos for $500. Scope: homepage, about page, logo. Any changes require written approval."
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
                Send this to the other person, save it in your email, screenshot it.
                Anyone with HSIP can paste it into "Verify a Proof" and confirm it's genuine.
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
              You type your message and click "Sign". HSIP creates a unique mathematical
              fingerprint of your exact words using a private key only your account holds.
            </p>
          </div>
          <div className="explainer-step">
            <span>2</span>
            <p>
              You get a Proof Certificate — a tamper-proof bundle with your message,
              a timestamp, and a signature that can only come from your key.
              Share it by email, screenshot, or paste.
            </p>
          </div>
          <div className="explainer-step">
            <span>3</span>
            <p>
              If anyone ever doubts you sent it — or claims you said something different —
              they paste the certificate into "Verify a Proof". Even one changed word
              breaks the proof. That's your alibi.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}
