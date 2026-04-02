import React, { useState, useEffect, useRef } from 'react';
import { request } from '../api';

// ── Helpers ───────────────────────────────────────────────────────────────────

function fmtTime(ms) {
  if (!ms) return '';
  const d = new Date(ms);
  const now = new Date();
  const isToday = d.toDateString() === now.toDateString();
  if (isToday) return d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
  return d.toLocaleDateString([], { month: 'short', day: 'numeric' }) +
    ' ' + d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' });
}

function fmtFull(ms) {
  return new Date(ms).toLocaleString([], {
    year: 'numeric', month: 'long', day: 'numeric',
    hour: '2-digit', minute: '2-digit', second: '2-digit', timeZoneName: 'short',
  });
}

function keyFp(b64) {
  if (!b64) return '';
  return b64.slice(0, 8) + '…' + b64.slice(-6);
}

function makeShareText(proof) {
  return `──── HSIP Signed Message ────
${proof.content}

From: ${keyFp(proof.sender_key)}
Signed: ${fmtFull(proof.signed_at_ms)}

[Paste this entire block into HSIP → Messages to verify]
HSIP_PROOF:${JSON.stringify({
    hsip_proof:    1,
    content:       proof.content,
    signature:     proof.signature,
    sender_key:    proof.sender_key,
    signed_at_ms:  proof.signed_at_ms,
    signed_at_iso: proof.signed_at_iso,
  })}
────────────────────────────`;
}

function parseProof(raw) {
  const s = raw.trim();
  const match = s.match(/HSIP_PROOF:(\{.+\})/s);
  if (match) { try { return JSON.parse(match[1]); } catch {} }
  try { const obj = JSON.parse(s); if (obj.hsip_proof) return obj; } catch {}
  return null;
}

// ── Add Contact Dialog ────────────────────────────────────────────────────────

function AddContactDialog({ apiKey, onAdded, onClose }) {
  const [nickname, setNickname] = useState('');
  const [key,      setKey]      = useState('');
  const [busy,     setBusy]     = useState(false);
  const [err,      setErr]      = useState('');

  async function save() {
    setErr('');
    if (!nickname.trim()) { setErr('Enter a name for this contact.'); return; }
    if (!key.trim())      { setErr('Paste their HSIP public key.'); return; }
    setBusy(true);
    try {
      const c = await request('POST', '/v1/contacts',
        { nickname: nickname.trim(), verify_key: key.trim() }, apiKey);
      onAdded(c);
      onClose();
    } catch (e) { setErr(e.message); }
    setBusy(false);
  }

  return (
    <div className="connect-dialog">
      <div className="connect-dialog-inner">
        <h3>Add a contact</h3>
        <p className="connect-hint">
          Ask the other person to open HSIP → Messages and tap <strong>"Share my address"</strong>.
          They copy their public key and send it to you — paste it below.
        </p>
        <label className="connect-label">Their name</label>
        <input className="connect-input" placeholder="e.g. Maria" value={nickname}
          onChange={e => setNickname(e.target.value)} autoFocus />
        <label className="connect-label">Their HSIP public key</label>
        <textarea className="connect-input" rows={3}
          placeholder="Paste their HSIP public key here…"
          value={key} onChange={e => setKey(e.target.value)}
          style={{ fontFamily: 'monospace', fontSize: '0.75rem', resize: 'vertical' }} />
        {err && <p style={{ color: '#fc8181', fontSize: '0.8rem', marginBottom: '0.5rem' }}>{err}</p>}
        <div className="connect-actions">
          <button className="consumer-reset-btn" onClick={onClose}>Cancel</button>
          <button className="primary" onClick={save} disabled={busy}>
            {busy ? 'Adding…' : 'Add contact'}
          </button>
        </div>
      </div>
    </div>
  );
}

// ── Receive dialog ────────────────────────────────────────────────────────────

function ReceiveDialog({ apiKey, contacts, onClose }) {
  const [raw,    setRaw]    = useState('');
  const [proof,  setProof]  = useState(null);
  const [result, setResult] = useState(null);
  const [busy,   setBusy]   = useState(false);
  const [err,    setErr]    = useState('');

  function tryParse(text) {
    setErr(''); setResult(null);
    const p = parseProof(text);
    if (p) setProof(p);
    else { setProof(null); if (text.length > 20) setErr('Could not read proof — paste the whole message including the HSIP_PROOF line.'); }
  }

  async function verify() {
    if (!proof) return;
    setBusy(true);
    try {
      const res = await request('POST', '/v1/messages/verify', {
        content:         proof.content,
        signature:       proof.signature,
        peer_verify_key: proof.sender_key,
      }, apiKey);
      setResult(res);
    } catch (e) { setErr(e.message); }
    setBusy(false);
  }

  const senderContact = proof && contacts.find(c => c.verify_key === proof.sender_key);

  return (
    <div className="connect-dialog">
      <div className="connect-dialog-inner" style={{ maxWidth: 520 }}>
        <h3>Receive a message</h3>
        <p className="connect-hint">Paste the full message you received from someone using HSIP.</p>
        <textarea className="connect-input" rows={6}
          placeholder="Paste the HSIP message here…"
          value={raw} onChange={e => { setRaw(e.target.value); tryParse(e.target.value); }}
          style={{ fontFamily: 'monospace', fontSize: '0.75rem', resize: 'vertical' }} />
        {err && <p style={{ color: '#fc8181', fontSize: '0.8rem' }}>{err}</p>}
        {proof && !result && (
          <div className="msg-preview-box">
            <p className="msg-preview-content">"{proof.content}"</p>
            <p className="msg-preview-meta">
              From: {senderContact ? <strong>{senderContact.nickname}</strong> : keyFp(proof.sender_key)}
              {proof.signed_at_ms && <> · {fmtFull(proof.signed_at_ms)}</>}
            </p>
          </div>
        )}
        {result && (
          <div className={`msg-verify-result ${result.verified ? 'msg-verify-ok' : 'msg-verify-fail'}`}>
            {result.verified
              ? '✅ Authentic — this message was not altered and came from the claimed sender.'
              : '❌ Verification failed — signature does not match.'}
          </div>
        )}
        <div className="connect-actions">
          <button className="consumer-reset-btn" onClick={onClose}>Close</button>
          {proof && !result && (
            <button className="primary" onClick={verify} disabled={busy}>
              {busy ? 'Verifying…' : 'Verify'}
            </button>
          )}
        </div>
      </div>
    </div>
  );
}

// ── Message bubble ────────────────────────────────────────────────────────────

function Bubble({ msg, myKey, contacts }) {
  const [open, setOpen] = useState(false);
  const isOut  = msg.direction === 'outbound';
  const proof  = {
    hsip_proof:    1,
    content:       msg.content,
    signature:     msg.signature,
    sender_key:    isOut ? myKey : msg.peer_verify_key,
    signed_at_ms:  msg.timestamp,
    signed_at_iso: new Date(msg.timestamp).toISOString(),
  };

  return (
    <div className={`bubble-row ${isOut ? 'bubble-row--out' : 'bubble-row--in'}`}>
      {!isOut && (
        <div className="bubble-avatar">
          {(contacts.find(c => c.verify_key === msg.peer_verify_key)?.nickname[0] || '?').toUpperCase()}
        </div>
      )}
      <div className="bubble-col">
        <div className={`bubble ${isOut ? 'bubble--out' : 'bubble--in'}`}
             onClick={() => setOpen(v => !v)}>
          <p className="bubble-text">{msg.content}</p>
          <div className="bubble-footer">
            <span className="bubble-time">{fmtTime(msg.timestamp)}</span>
            {isOut && <span className="bubble-status">{msg.verified ? '✓' : '○'}</span>}
          </div>
        </div>
        {open && (
          <div className="bubble-proof">
            <div className="bubble-proof-row">
              <span className="bubble-proof-label">Signed</span>
              <span>{fmtFull(msg.timestamp)}</span>
            </div>
            <div className="bubble-proof-row">
              <span className="bubble-proof-label">
                {isOut ? 'Your key' : 'Sender key'}
              </span>
              <code style={{ fontSize: '0.7rem' }}>
                {keyFp(isOut ? myKey : msg.peer_verify_key)}
              </code>
            </div>
            {!isOut && (
              <div className="bubble-proof-row">
                <span className="bubble-proof-label">Verified</span>
                <span style={{ color: msg.verified ? '#68d391' : '#fc8181' }}>
                  {msg.verified ? 'Yes ✓' : 'Failed ✗'}
                </span>
              </div>
            )}
            {isOut && (
              <button className="setup-copy-btn" style={{ marginTop: '0.4rem' }}
                onClick={() => navigator.clipboard.writeText(makeShareText(proof))}>
                Copy to share
              </button>
            )}
          </div>
        )}
      </div>
    </div>
  );
}

// ── Thread ────────────────────────────────────────────────────────────────────

function Thread({ contact, messages, myKey, apiKey, contacts, onSent }) {
  const [text,   setText]   = useState('');
  const [busy,   setBusy]   = useState(false);
  const [copied, setCopied] = useState(false);
  const bottomRef = useRef(null);

  const thread = messages
    .filter(m => m.peer_verify_key === contact.verify_key)
    .sort((a, b) => a.timestamp - b.timestamp);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: 'smooth' });
  }, [thread.length]);

  async function send() {
    if (!text.trim() || busy) return;
    setBusy(true);
    try {
      await request('POST', '/v1/messages/sign', {
        content:         text.trim(),
        peer_verify_key: contact.verify_key,
      }, apiKey);
      setText('');
      onSent();
    } catch (e) { alert(e.message); }
    setBusy(false);
  }

  const lastSent = [...thread].reverse().find(m => m.direction === 'outbound');
  function shareLastSent() {
    if (!lastSent) return;
    navigator.clipboard.writeText(makeShareText({
      content:       lastSent.content,
      signature:     lastSent.signature,
      sender_key:    myKey,
      signed_at_ms:  lastSent.timestamp,
      signed_at_iso: new Date(lastSent.timestamp).toISOString(),
    }));
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div className="thread">
      <div className="thread-header">
        <div className="thread-avatar">{contact.nickname[0].toUpperCase()}</div>
        <div>
          <div className="thread-name">{contact.nickname}</div>
          <div className="thread-key">{keyFp(contact.verify_key)}</div>
        </div>
      </div>

      <div className="thread-messages">
        {thread.length === 0 && (
          <div className="thread-empty">
            <p>No messages yet with {contact.nickname}.</p>
            <p>Type below, sign it, then share the proof with them so they can verify it.</p>
          </div>
        )}
        {thread.map(m => (
          <Bubble key={m.id} msg={m} myKey={myKey} contacts={contacts} />
        ))}
        <div ref={bottomRef} />
      </div>

      <div className="thread-compose">
        {lastSent && (
          <button className="share-btn" onClick={shareLastSent}>
            {copied ? '✓ Copied!' : '📋 Copy last message to share'}
          </button>
        )}
        <div className="compose-row">
          <textarea className="compose-input"
            placeholder={`Message ${contact.nickname}…`}
            value={text} rows={2}
            onChange={e => setText(e.target.value)}
            onKeyDown={e => { if (e.key === 'Enter' && !e.shiftKey) { e.preventDefault(); send(); } }}
          />
          <button className="compose-send" onClick={send} disabled={busy || !text.trim()}>
            {busy ? '…' : '✍️'}
          </button>
        </div>
        <p className="compose-hint">
          Tap ✍️ to sign. Then <strong>Copy last message to share</strong> and send it to {contact.nickname} via any channel — they paste it into HSIP to verify.
        </p>
      </div>
    </div>
  );
}

// ── My address ────────────────────────────────────────────────────────────────

function MyAddress({ myKey }) {
  const [copied, setCopied] = useState(false);
  function copy() {
    navigator.clipboard.writeText(myKey);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }
  return (
    <div className="my-address">
      <div className="my-address-label">Your HSIP address</div>
      <div className="my-address-key">{keyFp(myKey)}</div>
      <button className="my-address-copy" onClick={copy}>
        {copied ? '✓ Copied!' : 'Share my address'}
      </button>
    </div>
  );
}

// ── Main ──────────────────────────────────────────────────────────────────────

export default function Messages({ apiKey }) {
  const [myKey,       setMyKey]       = useState('');
  const [contacts,    setContacts]    = useState([]);
  const [messages,    setMessages]    = useState([]);
  const [selected,    setSelected]    = useState(null);
  const [showAdd,     setShowAdd]     = useState(false);
  const [showReceive, setShowReceive] = useState(false);
  const [loading,     setLoading]     = useState(true);

  useEffect(() => { loadAll(); }, []);

  async function loadAll() {
    try {
      const [identity, contactList, msgList] = await Promise.all([
        request('GET', '/v1/identity', null, apiKey)
          .catch(() => request('POST', '/v1/identity', null, apiKey)),
        request('GET', '/v1/contacts', null, apiKey),
        request('GET', '/v1/messages', null, apiKey),
      ]);
      setMyKey(identity.verify_key_b64 || '');
      setContacts(Array.isArray(contactList) ? contactList : []);
      setMessages(Array.isArray(msgList)     ? msgList     : []);
    } catch {}
    setLoading(false);
  }

  const selectedContact = contacts.find(c => c.id === selected);

  if (loading) return <div className="card"><p className="empty">Loading…</p></div>;

  return (
    <div className="messages-shell">
      {showAdd && (
        <AddContactDialog
          apiKey={apiKey}
          onAdded={c => {
            setContacts(prev => [...prev.filter(x => x.id !== c.id), c]
              .sort((a, b) => a.nickname.localeCompare(b.nickname)));
            setSelected(c.id);
          }}
          onClose={() => setShowAdd(false)}
        />
      )}
      {showReceive && (
        <ReceiveDialog apiKey={apiKey} contacts={contacts}
          onClose={() => { setShowReceive(false); loadAll(); }} />
      )}

      {/* Sidebar */}
      <div className="msg-sidebar">
        {myKey && <MyAddress myKey={myKey} />}
        <div className="sidebar-actions">
          <button className="sidebar-btn sidebar-btn--primary" onClick={() => setShowAdd(true)}>
            + Add contact
          </button>
          <button className="sidebar-btn" onClick={() => setShowReceive(true)}>
            📥 Receive
          </button>
        </div>
        <div className="contact-list">
          {contacts.length === 0 && (
            <p className="contact-empty">No contacts yet.<br />Add someone to start.</p>
          )}
          {contacts.map(c => {
            const last = [...messages]
              .filter(m => m.peer_verify_key === c.verify_key)
              .sort((a, b) => b.timestamp - a.timestamp)[0];
            return (
              <button key={c.id}
                className={`contact-item${selected === c.id ? ' contact-item--active' : ''}`}
                onClick={() => setSelected(c.id)}>
                <div className="contact-avatar">{c.nickname[0].toUpperCase()}</div>
                <div className="contact-info">
                  <div className="contact-name">{c.nickname}</div>
                  <div className="contact-preview">
                    {last
                      ? last.content.slice(0, 38) + (last.content.length > 38 ? '…' : '')
                      : 'No messages yet'}
                  </div>
                </div>
              </button>
            );
          })}
        </div>
      </div>

      {/* Main */}
      <div className="msg-main">
        {!selectedContact ? (
          <div className="msg-empty-state">
            <div style={{ fontSize: '3rem', marginBottom: '1rem' }}>💬</div>
            <h3>Secure Messages</h3>
            <p>
              Every message is signed with your private key — mathematical proof
              of exactly what was said and when. Useful in disputes, contracts, or court.
            </p>
            <div style={{ display: 'flex', gap: '0.75rem', marginTop: '1.25rem', justifyContent: 'center', flexWrap: 'wrap' }}>
              <button className="primary" onClick={() => setShowAdd(true)}>
                + Add a contact
              </button>
              <button className="consumer-reset-btn" onClick={() => setShowReceive(true)}>
                📥 Receive a message
              </button>
            </div>
          </div>
        ) : (
          <Thread
            contact={selectedContact}
            messages={messages}
            myKey={myKey}
            apiKey={apiKey}
            contacts={contacts}
            onSent={loadAll}
          />
        )}
      </div>
    </div>
  );
}
