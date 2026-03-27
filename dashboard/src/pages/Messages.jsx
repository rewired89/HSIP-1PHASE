import React, { useState, useEffect } from 'react';
import { request } from '../api';

/** Full human-readable timestamp, e.g. "March 26, 2024 at 03:33:20 PM EST" */
function fmtFull(ms) {
  return new Date(ms).toLocaleString('en-US', {
    year: 'numeric', month: 'long', day: 'numeric',
    hour: '2-digit', minute: '2-digit', second: '2-digit',
    timeZoneName: 'short',
  });
}

/** ISO 8601 for technical / legal export */
function fmtISO(ms) {
  return new Date(ms).toISOString();
}

/** Show first 10 chars + … + last 8 chars of a base64 key */
function keyFp(b64) {
  if (!b64 || b64.length < 20) return b64 || '—';
  return b64.slice(0, 10) + '…' + b64.slice(-8);
}

export default function Messages({ apiKey }) {
  const [tab,      setTab]      = useState('send');
  const [myKey,    setMyKey]    = useState('');
  const [messages, setMessages] = useState([]);

  // ── Compose state ────────────────────────────────────────────────────────
  const [content,      setContent]      = useState('');
  const [proofPackage, setProofPackage] = useState(null);
  const [copiedProof,  setCopiedProof]  = useState(false);
  const [copiedKey,    setCopiedKey]    = useState(false);
  const [signing,      setSigning]      = useState(false);

  // ── Receive state ────────────────────────────────────────────────────────
  const [pasted,       setPasted]       = useState('');
  const [parsed,       setParsed]       = useState(null);
  const [parseError,   setParseError]   = useState('');
  const [verifyResult, setVerifyResult] = useState(null);
  const [verifying,    setVerifying]    = useState(false);

  // ── History state ────────────────────────────────────────────────────────
  const [expandedId, setExpandedId] = useState(null);

  useEffect(() => {
    loadIdentity();
    loadMessages();
  }, []);

  async function loadIdentity() {
    try {
      const r = await request('POST', '/v1/identity', null, apiKey);
      setMyKey(r.verify_key);
    } catch {}
  }

  async function loadMessages() {
    try { setMessages(await request('GET', '/v1/messages', null, apiKey)); } catch {}
  }

  // ── Sign ─────────────────────────────────────────────────────────────────
  async function handleSign() {
    if (!content.trim()) return;
    setSigning(true);
    try {
      const r = await request('POST', '/v1/messages/sign', { content: content.trim() }, apiKey);
      setProofPackage({
        hsip_proof:    1,
        content:       r.content,
        signature:     r.signature,
        sender_key:    myKey,
        signed_at_ms:  r.timestamp,
        signed_at_iso: fmtISO(r.timestamp),
        signed_at:     fmtFull(r.timestamp),
      });
      loadMessages();
    } catch (e) {
      alert('Could not sign the message. Please try again.\n\nDetail: ' + e.message);
    }
    setSigning(false);
  }

  function copyProof() {
    navigator.clipboard.writeText(JSON.stringify(proofPackage, null, 2));
    setCopiedProof(true);
    setTimeout(() => setCopiedProof(false), 2500);
  }

  function copyKey() {
    navigator.clipboard.writeText(myKey);
    setCopiedKey(true);
    setTimeout(() => setCopiedKey(false), 2500);
  }

  // ── Verify ───────────────────────────────────────────────────────────────
  function handlePaste(text) {
    setPasted(text);
    setParseError('');
    setParsed(null);
    setVerifyResult(null);
    if (!text.trim()) return;
    try {
      const obj = JSON.parse(text.trim());
      if (obj.hsip_proof !== 1 || !obj.content || !obj.signature || !obj.sender_key) {
        setParseError(
          'This doesn\'t look like a complete HSIP proof package. ' +
          'Make sure you copied the entire block — from the opening { to the closing }.'
        );
        return;
      }
      setParsed(obj);
    } catch {
      setParseError(
        'Could not read this text. Make sure you copied the entire proof package your contact sent, ' +
        'starting with { and ending with }.'
      );
    }
  }

  async function handleVerify() {
    if (!parsed) return;
    setVerifying(true);
    try {
      const r = await request('POST', '/v1/messages/verify', {
        content:        parsed.content,
        signature:      parsed.signature,
        peer_verify_key: parsed.sender_key,
      }, apiKey);
      setVerifyResult({ ...r, pkg: parsed });
      loadMessages();
    } catch (e) {
      alert('Verification request failed. Please try again.\n\nDetail: ' + e.message);
    }
    setVerifying(false);
  }

  function resetReceive() {
    setPasted('');
    setParsed(null);
    setParseError('');
    setVerifyResult(null);
  }

  // ── Print ────────────────────────────────────────────────────────────────
  function handlePrint() {
    window.print();
  }

  // ─────────────────────────────────────────────────────────────────────────
  return (
    <div>
      {/* ── Tab bar ──────────────────────────────────────────────────────── */}
      <div className="msg-tabs">
        <button className={tab === 'send'    ? 'active' : ''} onClick={() => setTab('send')}>
          ✍️ Send a Message
        </button>
        <button className={tab === 'receive' ? 'active' : ''} onClick={() => setTab('receive')}>
          📥 Receive &amp; Verify
        </button>
        <button className={tab === 'history' ? 'active' : ''} onClick={() => setTab('history')}>
          📋 History
          {messages.length > 0 && <span className="msg-count">{messages.length}</span>}
        </button>
      </div>

      {/* ══ SEND TAB ═══════════════════════════════════════════════════════ */}
      {tab === 'send' && (
        <div>
          {/* My identity key */}
          {myKey && (
            <div className="card msg-identity-card">
              <div className="msg-identity-label">
                <span>🔑</span>
                <div>
                  <strong>Your Identity Key — share this with your contacts</strong>
                  <p>
                    Your contact needs this key so they can verify messages came from you.
                    Share it once via text, email, or any method — you only need to do it once.
                  </p>
                </div>
              </div>
              <div className="key-display">{myKey}</div>
              <button className="secondary" style={{ marginTop: '0.6rem' }} onClick={copyKey}>
                {copiedKey ? '✓ Copied!' : 'Copy My Identity Key'}
              </button>
            </div>
          )}

          {!proofPackage ? (
            <div className="card">
              <h2>Write &amp; Sign a Message</h2>
              <p className="msg-subtitle">
                Type your message below. HSIP will sign it with your private key, creating a
                cryptographic proof that it came from you — and only you — at this exact time.
                <br /><br />
                After signing, you will get a <strong>Proof Package</strong> to send to your contact.
                They paste it into HSIP to confirm it is real and unmodified.
              </p>
              <label className="msg-label">Your message</label>
              <textarea
                rows={6}
                placeholder="Type your message here. Be as detailed as you like — include names, dates, and amounts if relevant."
                value={content}
                onChange={e => setContent(e.target.value)}
              />
              <button
                className="primary"
                onClick={handleSign}
                disabled={signing || !content.trim()}
              >
                {signing ? 'Signing…' : '✍️ Sign & Create Proof Package'}
              </button>
            </div>
          ) : (
            <div className="card msg-proof-card">
              <h2 style={{ color: '#68d391' }}>✅ Message Signed</h2>
              <p className="msg-subtitle">
                Copy the entire block below and send it to your contact however you like —
                text message, email, WhatsApp, etc. They paste it into HSIP to confirm it
                is genuinely from you and has not been changed.
              </p>

              <div className="msg-proof-meta">
                <div><span>Signed at</span><strong>{proofPackage.signed_at}</strong></div>
                <div><span>ISO timestamp</span><strong>{proofPackage.signed_at_iso}</strong></div>
              </div>

              <label className="msg-label" style={{ marginTop: '0.75rem' }}>Proof Package — copy and send this</label>
              <textarea
                className="msg-proof-textarea"
                rows={12}
                readOnly
                value={JSON.stringify(proofPackage, null, 2)}
              />

              <div style={{ display: 'flex', gap: '0.6rem', marginTop: '0.75rem', flexWrap: 'wrap' }}>
                <button className="primary" onClick={copyProof}>
                  {copiedProof ? '✓ Copied!' : '📋 Copy Proof Package'}
                </button>
                <button className="secondary" onClick={() => { setProofPackage(null); setContent(''); }}>
                  Write Another Message
                </button>
              </div>
            </div>
          )}
        </div>
      )}

      {/* ══ RECEIVE TAB ════════════════════════════════════════════════════ */}
      {tab === 'receive' && (
        <div>
          {!verifyResult ? (
            <div className="card">
              <h2>Receive &amp; Verify a Message</h2>
              <p className="msg-subtitle">
                When your contact sends you a Proof Package, paste the entire thing below.
                HSIP will check that the message is genuine, has not been changed, and was
                really signed by their key.
              </p>

              <label className="msg-label">Paste the Proof Package here</label>
              <textarea
                className="msg-proof-textarea"
                rows={10}
                placeholder={'Paste the proof package your contact sent you.\n\nIt looks like this:\n{\n  "hsip_proof": 1,\n  "content": "...",\n  ...\n}'}
                value={pasted}
                onChange={e => handlePaste(e.target.value)}
              />

              {parseError && (
                <p className="msg-error">{parseError}</p>
              )}

              {parsed && (
                <div className="msg-preview">
                  <p className="msg-preview-label">Preview — message from your contact:</p>
                  <div className="msg-bubble">{parsed.content}</div>
                  <div className="msg-preview-meta">
                    <span>
                      Signed at: <strong>
                        {parsed.signed_at || (parsed.signed_at_ms ? fmtFull(parsed.signed_at_ms) : '(unknown)')}
                      </strong>
                    </span>
                    <span>
                      Sender key: <code>{keyFp(parsed.sender_key)}</code>
                    </span>
                  </div>
                </div>
              )}

              <div style={{ display: 'flex', gap: '0.6rem', marginTop: '0.75rem', flexWrap: 'wrap' }}>
                <button
                  className="primary"
                  onClick={handleVerify}
                  disabled={verifying || !parsed}
                >
                  {verifying ? 'Verifying…' : '🔍 Verify Message'}
                </button>
                {pasted && (
                  <button className="secondary" onClick={resetReceive}>Clear</button>
                )}
              </div>
            </div>
          ) : (
            <div className="card" style={{ borderColor: verifyResult.verified ? '#38a169' : '#fc8181' }}>
              {verifyResult.verified ? (
                <>
                  <h2 style={{ color: '#68d391', textTransform: 'none', letterSpacing: 0 }}>
                    ✅ Authentic — Signature Valid
                  </h2>
                  <p className="msg-subtitle">
                    The cryptographic signature on this message is valid. It was signed by the
                    key shown below and has not been altered since. This is safe to trust.
                  </p>

                  <div className="msg-verified-block">
                    <div className="msg-verified-row">
                      <span>Message</span>
                      <div className="msg-bubble msg-bubble--verified">{verifyResult.pkg.content}</div>
                    </div>
                    <div className="msg-verified-row">
                      <span>Signed at</span>
                      <strong>
                        {verifyResult.pkg.signed_at || fmtFull(verifyResult.pkg.signed_at_ms)}
                      </strong>
                    </div>
                    <div className="msg-verified-row">
                      <span>ISO timestamp</span>
                      <code>{verifyResult.pkg.signed_at_iso || fmtISO(verifyResult.pkg.signed_at_ms)}</code>
                    </div>
                    <div className="msg-verified-row">
                      <span>Sender key</span>
                      <code className="msg-key-full">{verifyResult.pkg.sender_key}</code>
                    </div>
                    <div className="msg-verified-row">
                      <span>Signature</span>
                      <code className="msg-key-full">{verifyResult.pkg.signature}</code>
                    </div>
                  </div>
                </>
              ) : (
                <>
                  <h2 style={{ color: '#fc8181', textTransform: 'none', letterSpacing: 0 }}>
                    ❌ Verification Failed — Do Not Trust This Message
                  </h2>
                  <p className="msg-subtitle">
                    The signature does not match the content. Either the message was tampered with
                    after it was signed, or it was not signed by the key it claims to be from.
                    Do not act on this message.
                  </p>
                </>
              )}
              <button className="secondary" style={{ marginTop: '1rem' }} onClick={resetReceive}>
                Verify Another Message
              </button>
            </div>
          )}
        </div>
      )}

      {/* ══ HISTORY TAB ════════════════════════════════════════════════════ */}
      {tab === 'history' && (
        <div>
          <div className="card no-print" style={{ marginBottom: '0.75rem' }}>
            <div style={{ display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
              <div>
                <h2 style={{ margin: 0 }}>Message History</h2>
                <p className="msg-subtitle" style={{ marginTop: '0.3rem', marginBottom: 0 }}>
                  Every message signed or verified on this device. Click any row to expand.
                  Use <strong>Print / Export</strong> to save a clean record for your files or for court.
                </p>
              </div>
              <button className="secondary" onClick={handlePrint} style={{ flexShrink: 0, marginLeft: '1rem' }}>
                🖨️ Print / Export
              </button>
            </div>
          </div>

          {/* Print header — only shows on paper */}
          <div className="print-only msg-print-header">
            <h1>HSIP Signed Message Record</h1>
            <p>Exported: {fmtFull(Date.now())}</p>
            <p>This document lists cryptographically signed messages. Each signature can be independently verified using the HSIP verify function and the sender's identity key.</p>
            <hr />
          </div>

          {messages.length === 0 ? (
            <div className="card">
              <p className="empty">No messages yet. Sign or verify a message to see it here.</p>
            </div>
          ) : messages.map(m => (
            <div
              key={m.id}
              className={'card msg-history-row' + (expandedId === m.id ? ' msg-history-row--open' : '')}
              onClick={() => setExpandedId(expandedId === m.id ? null : m.id)}
            >
              <div className="msg-history-top">
                <span className="msg-dir-icon">
                  {m.direction === 'outbound' ? '📤' : '📥'}
                </span>
                <div className="msg-history-meta">
                  <div className="msg-history-badges">
                    <span className={'badge ' + (m.direction === 'outbound' ? 'granted' : 'verified')}>
                      {m.direction === 'outbound' ? 'Sent' : 'Received'}
                    </span>
                    <span className={'badge ' + (m.verified ? 'verified' : 'failed')}>
                      {m.verified ? '✓ Verified' : '✗ Unverified'}
                    </span>
                    <span className="msg-timestamp">{fmtFull(m.timestamp)}</span>
                  </div>
                  <p className={'msg-preview-text' + (expandedId === m.id ? ' msg-preview-text--expanded' : '')}>
                    {m.content}
                  </p>
                </div>
                <span className="msg-chevron">{expandedId === m.id ? '▲' : '▼'}</span>
              </div>

              {expandedId === m.id && (
                <div className="msg-history-detail">
                  <div className="msg-verified-block">
                    <div className="msg-verified-row">
                      <span>Full message</span>
                      <div className="msg-bubble">{m.content}</div>
                    </div>
                    <div className="msg-verified-row">
                      <span>Signed at (human)</span>
                      <strong>{fmtFull(m.timestamp)}</strong>
                    </div>
                    <div className="msg-verified-row">
                      <span>Signed at (ISO 8601)</span>
                      <code>{fmtISO(m.timestamp)}</code>
                    </div>
                    <div className="msg-verified-row">
                      <span>Unix timestamp (ms)</span>
                      <code>{m.timestamp}</code>
                    </div>
                    <div className="msg-verified-row">
                      <span>{m.direction === 'outbound' ? 'Recipient key' : 'Sender key'}</span>
                      <code className="msg-key-full">{m.peer_verify_key || '(not set)'}</code>
                    </div>
                    <div className="msg-verified-row">
                      <span>Signature</span>
                      <code className="msg-key-full">{m.signature}</code>
                    </div>
                  </div>
                </div>
              )}

              {/* Always visible on paper */}
              <div className="print-only msg-print-row">
                <table>
                  <tbody>
                    <tr><td><strong>Direction</strong></td><td>{m.direction} ({m.verified ? 'verified' : 'unverified'})</td></tr>
                    <tr><td><strong>Timestamp</strong></td><td>{fmtFull(m.timestamp)}</td></tr>
                    <tr><td><strong>ISO 8601</strong></td><td>{fmtISO(m.timestamp)}</td></tr>
                    <tr><td><strong>Content</strong></td><td style={{ whiteSpace: 'pre-wrap' }}>{m.content}</td></tr>
                    <tr><td><strong>Peer key</strong></td><td style={{ wordBreak: 'break-all', fontFamily: 'monospace', fontSize: '0.75rem' }}>{m.peer_verify_key || '—'}</td></tr>
                    <tr><td><strong>Signature</strong></td><td style={{ wordBreak: 'break-all', fontFamily: 'monospace', fontSize: '0.75rem' }}>{m.signature}</td></tr>
                  </tbody>
                </table>
                <hr />
              </div>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
