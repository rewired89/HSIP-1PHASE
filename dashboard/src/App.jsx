import React, { useState, useEffect } from 'react';
import { request } from './api';
import Identity    from './pages/Identity';
import Consent     from './pages/Consent';
import Messages    from './pages/Messages';
import Audit       from './pages/Audit';
import Keys        from './pages/Keys';
import Credentials from './pages/Credentials';
import './App.css';

export default function App() {
  const [apiKey,  setApiKey]  = useState(localStorage.getItem('hsip_api_key') || '');
  const [tab,     setTab]     = useState('identity');
  const [authed,  setAuthed]  = useState(false);
  const [error,   setError]   = useState('');

  async function handleLogin(e) {
    e.preventDefault();
    try {
      await request('POST', '/v1/identity', null, apiKey);
      localStorage.setItem('hsip_api_key', apiKey);
      setAuthed(true);
      setError('');
    } catch {
      setError('Invalid API key');
    }
  }

  if (!authed) {
    return (
      <div className="login-screen">
        <div className="login-card">
          <h1>🔐 HSIP Dashboard</h1>
          <p>Cryptographic consent &amp; message verification</p>
          <form onSubmit={handleLogin}>
            <input
              type="text"
              placeholder="Enter API key (hsip_...)"
              value={apiKey}
              onChange={e => setApiKey(e.target.value)}
            />
            <button type="submit">Connect</button>
          </form>
          {error && <p className="error">{error}</p>}
        </div>
      </div>
    );
  }

  const tabs = ['identity', 'consent', 'messages', 'credentials', 'audit', 'keys'];

  return (
    <div className="app">
      <header>
        <h1>HSIP Dashboard</h1>
        <nav>
          {tabs.map(t => (
            <button
              key={t}
              className={tab === t ? 'active' : ''}
              onClick={() => setTab(t)}
            >
              {t.charAt(0).toUpperCase() + t.slice(1)}
            </button>
          ))}
          <button onClick={() => { localStorage.removeItem('hsip_api_key'); setAuthed(false); }}>
            Logout
          </button>
        </nav>
      </header>
      <main>
        {tab === 'identity'    && <Identity    apiKey={apiKey} />}
        {tab === 'consent'     && <Consent     apiKey={apiKey} />}
        {tab === 'messages'    && <Messages    apiKey={apiKey} />}
        {tab === 'credentials' && <Credentials apiKey={apiKey} />}
        {tab === 'audit'       && <Audit       apiKey={apiKey} />}
        {tab === 'keys'        && <Keys        apiKey={apiKey} />}
      </main>
    </div>
  );
}
