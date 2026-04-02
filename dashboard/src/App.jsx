import React, { useState } from 'react';
import { request } from './api';
import Identity        from './pages/Identity';
import Consent         from './pages/Consent';
import Messages        from './pages/Messages';
import Audit           from './pages/Audit';
import Keys            from './pages/Keys';
import Credentials     from './pages/Credentials';
import HomeDashboard   from './pages/HomeDashboard';
import ProveIt         from './pages/ProveIt';
import ConsentWallet   from './pages/ConsentWallet';
import AIWatch         from './pages/AIWatch';
import TrackerInspector from './pages/TrackerInspector';
import ProtectionSetup  from './pages/ProtectionSetup';
import NetworkMonitor   from './pages/NetworkMonitor';
import Onboarding       from './pages/Onboarding';
import './App.css';

const SIMPLE_TABS = [
  { id: 'home',              label: '🏠 Home' },
  { id: 'messages',          label: '💬 Messages' },
  { id: 'network',           label: '🌐 Network' },
  { id: 'prove-it',          label: '✍️ Alibi' },
  { id: 'consent-wallet',    label: '🛡️ My Consents' },
  { id: 'ai-watch',          label: '🤖 AI Watch' },
  { id: 'tracker-inspector', label: '🔍 Trackers' },
  { id: 'protection',        label: '🔒 Protection' },
];

const EXPERT_TABS = ['identity', 'consent', 'messages', 'credentials', 'audit', 'keys'];

export default function App() {
  const [apiKey,    setApiKey]    = useState(localStorage.getItem('hsip_api_key') || '');
  const [authed,    setAuthed]    = useState(false);
  const [error,     setError]     = useState('');
  const [mode,      setMode]      = useState(localStorage.getItem('hsip_mode') || 'simple');
  const [tab,       setTab]       = useState(
    localStorage.getItem('hsip_mode') === 'expert' ? 'identity' : 'home'
  );
  // Show onboarding if user has never completed it
  const [onboarding, setOnboarding] = useState(false);

  function switchMode(m) {
    setMode(m);
    localStorage.setItem('hsip_mode', m);
    setTab(m === 'simple' ? 'home' : 'identity');
  }

  async function handleLogin(e) {
    e.preventDefault();
    try {
      await request('POST', '/v1/identity', null, apiKey);
      localStorage.setItem('hsip_api_key', apiKey);
      setAuthed(true);
      setError('');
      // Show onboarding on first ever login (simple mode only)
      if (!localStorage.getItem('hsip_onboarding_done') && mode === 'simple') {
        setOnboarding(true);
      }
      setTab(mode === 'simple' ? 'home' : 'identity');
    } catch {
      setError('Invalid access key. Please check and try again.');
    }
  }

  function logout() {
    localStorage.removeItem('hsip_api_key');
    setAuthed(false);
  }

  if (!authed) {
    return (
      <div className="login-screen">
        <div className="login-card">
          <div className="login-logo">🔐</div>
          <h1>HSIP</h1>
          <p>{mode === 'simple' ? 'Your personal data security hub' : 'Cryptographic consent & verification'}</p>

          <div className="mode-toggle-login">
            <button
              className={mode === 'simple' ? 'active' : ''}
              onClick={() => switchMode('simple')}
            >
              For Everyone
            </button>
            <button
              className={mode === 'expert' ? 'active' : ''}
              onClick={() => switchMode('expert')}
            >
              Developer Mode
            </button>
          </div>

          <form onSubmit={handleLogin}>
            <input
              type="text"
              placeholder={mode === 'simple' ? 'Enter your access key (hsip_…)' : 'Enter API key (hsip_…)'}
              value={apiKey}
              onChange={e => setApiKey(e.target.value)}
            />
            <button type="submit">{mode === 'simple' ? 'Enter' : 'Connect'}</button>
          </form>
          {error && <p className="error">{error}</p>}
          <div className="login-key-hint">
            <p className="login-hint-head">First time? Your access key was shown in the terminal when HSIP first started, and saved to:</p>
            <div className="login-hint-paths">
              <span className="login-hint-os">Windows</span>
              <code>%APPDATA%\HSIP\admin.key</code>
              <span className="login-hint-os">Mac&nbsp;/&nbsp;Linux</span>
              <code>~/.hsip/admin.key</code>
            </div>
          </div>
        </div>
      </div>
    );
  }

  return (
    <div className="app">
      {/* First-run onboarding overlay */}
      {onboarding && (
        <Onboarding onComplete={() => setOnboarding(false)} />
      )}

      <header>
        <h1 className="app-title">
          HSIP {mode === 'expert' && <span className="mode-badge">Dev</span>}
        </h1>
        <nav>
          {mode === 'simple'
            ? SIMPLE_TABS.map(t => (
                <button
                  key={t.id}
                  className={tab === t.id ? 'active' : ''}
                  onClick={() => setTab(t.id)}
                >
                  {t.label}
                </button>
              ))
            : EXPERT_TABS.map(t => (
                <button
                  key={t}
                  className={tab === t ? 'active' : ''}
                  onClick={() => setTab(t)}
                >
                  {t.charAt(0).toUpperCase() + t.slice(1)}
                </button>
              ))
          }
          <div className="nav-right">
            {mode === 'simple' && (
              <button
                className="mode-switch-btn ob-replay-btn"
                onClick={() => setOnboarding(true)}
                title="What does HSIP do on my machine?"
              >
                ❓ What is HSIP?
              </button>
            )}
            <button
              className="mode-switch-btn"
              onClick={() => switchMode(mode === 'simple' ? 'expert' : 'simple')}
              title={mode === 'simple' ? 'Switch to Developer Mode' : 'Switch to Simple Mode'}
            >
              {mode === 'simple' ? '⚙️ Dev Mode' : '👤 Simple Mode'}
            </button>
            <button onClick={logout}>Logout</button>
          </div>
        </nav>
      </header>

      <main>
        {mode === 'simple' ? (
          <>
            {tab === 'home'              && <HomeDashboard    onNavigate={setTab} />}
            {tab === 'messages'          && <Messages         apiKey={apiKey} />}
            {tab === 'network'           && <NetworkMonitor   apiKey={apiKey} />}
            {tab === 'prove-it'          && <ProveIt          apiKey={apiKey} />}
            {tab === 'consent-wallet'    && <ConsentWallet    apiKey={apiKey} />}
            {tab === 'ai-watch'          && <AIWatch          apiKey={apiKey} />}
            {tab === 'tracker-inspector' && <TrackerInspector />}
            {tab === 'protection'        && <ProtectionSetup  apiKey={apiKey} />}
          </>
        ) : (
          <>
            {tab === 'identity'    && <Identity    apiKey={apiKey} />}
            {tab === 'consent'     && <Consent     apiKey={apiKey} />}
            {tab === 'messages'    && <Messages    apiKey={apiKey} />}
            {tab === 'credentials' && <Credentials apiKey={apiKey} />}
            {tab === 'audit'       && <Audit       apiKey={apiKey} />}
            {tab === 'keys'        && <Keys        apiKey={apiKey} />}
          </>
        )}
      </main>
    </div>
  );
}
