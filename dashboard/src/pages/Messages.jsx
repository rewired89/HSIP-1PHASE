import React, { useState, useEffect } from 'react';
import { request } from '../api';

export default function Messages({ apiKey }) {
  const [messages, setMessages] = useState([]);
  const [content,  setContent]  = useState('');
  const [peerKey,  setPeerKey]  = useState('');
  const [signed,   setSigned]   = useState(null);
  const [vContent, setVContent] = useState('');
  const [vSig,     setVSig]     = useState('');
  const [vPeer,    setVPeer]    = useState('');
  const [vResult,  setVResult]  = useState(null);

  useEffect(() => { loadMessages(); }, []);

  async function loadMessages() {
    try { setMessages(await request('GET', '/v1/messages', null, apiKey)); } catch {}
  }

  async function sign() {
    try {
      const body = { content };
      if (peerKey) body.peer_verify_key = peerKey;
      const r = await request('POST', '/v1/messages/sign', body, apiKey);
      setSigned(r);
      loadMessages();
    } catch (e) { alert(e.message); }
  }

  async function verify() {
    try {
      const r = await request('POST', '/v1/messages/verify',
        { content: vContent, signature: vSig, peer_verify_key: vPeer }, apiKey);
      setVResult(r);
      loadMessages();
    } catch (e) { alert(e.message); }
  }

  return (
    <div>
      <div className="card">
        <h2>Sign Message</h2>
        <textarea rows={3} placeholder="Message content..." value={content} onChange={e => setContent(e.target.value)} />
        <input placeholder="Peer verify key (optional)" value={peerKey} onChange={e => setPeerKey(e.target.value)} />
        <button className="primary" onClick={sign}>Sign</button>
        {signed && (
          <div style={{ marginTop: '1rem' }}>
            <p style={{ color: '#68d391', marginBottom: '0.5rem' }}>Signed successfully</p>
            <div className="key-display">Signature: {signed.signature}</div>
          </div>
        )}
      </div>
      <div className="card">
        <h2>Verify Message</h2>
        <textarea rows={3} placeholder="Message content..." value={vContent} onChange={e => setVContent(e.target.value)} />
        <input placeholder="Signature (base64)" value={vSig} onChange={e => setVSig(e.target.value)} />
        <input placeholder="Peer verify key" value={vPeer} onChange={e => setVPeer(e.target.value)} />
        <button className="primary" onClick={verify}>Verify</button>
        {vResult && (
          <p style={{ marginTop: '0.75rem', color: vResult.verified ? '#68d391' : '#fc8181' }}>
            {vResult.verified
              ? 'Signature valid — message is authentic'
              : 'Invalid signature — message may be forged'}
          </p>
        )}
      </div>
      <div className="card">
        <h2>Message Log</h2>
        {messages.length === 0
          ? <p className="empty">No messages yet.</p>
          : (
            <table>
              <thead><tr><th>Direction</th><th>Content</th><th>Verified</th><th>Time</th></tr></thead>
              <tbody>
                {messages.map(m => (
                  <tr key={m.id}>
                    <td>{m.direction}</td>
                    <td style={{ maxWidth: 200, overflow: 'hidden', textOverflow: 'ellipsis', whiteSpace: 'nowrap' }}>{m.content}</td>
                    <td><span className={'badge ' + (m.verified ? 'verified' : 'failed')}>{m.verified ? 'yes' : 'no'}</span></td>
                    <td style={{ fontSize: '0.8rem' }}>{new Date(m.timestamp).toLocaleString()}</td>
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
