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
  { id: 'home',              icon: '🏠', label: 'Home' },
  { id: 'messages',          icon: '💬', label: 'Messages' },
  { id: 'network',           icon: '🌐', label: 'Traffic' },
  { id: 'prove-it',          icon: '✍️',  label: 'Alibi' },
  { id: 'consent-wallet',    icon: '🛡️',  label: 'Consents' },
  { id: 'ai-watch',          icon: '🤖', label: 'AI Watch' },
  { id: 'tracker-inspector', icon: '🔍', label: 'Trackers' },
  { id: 'protection',        icon: '🔒', label: 'Protection' },
];

const EXPERT_TABS = [
  { id: 'identity',    icon: '🆔', label: 'Identity' },
  { id: 'consent',     icon: '✅', label: 'Consent' },
  { id: 'messages',    icon: '💬', label: 'Messages' },
  { id: 'credentials', icon: '🏅', label: 'Credentials' },
  { id: 'audit',       icon: '📋', label: 'Audit' },
  { id: 'keys',        icon: '🔑', label: 'Keys' },
];

export default function App() {
  const [apiKey,    setApiKey]    = useState(localStorage.getItem('hsip_api_key') || '');
  const [authed,    setAuthed]    = useState(false);
  const [error,     setError]     = useState('');
  const [mode,      setMode]      = useState(localStorage.getItem('hsip_mode') || 'simple');
  const [tab,       setTab]       = useState(
    localStorage.getItem('hsip_mode') === 'expert' ? 'identity' : 'home'
  );
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
          <p className="login-tagline">
            {mode === 'simple' ? 'Your personal security hub' : 'Cryptographic identity & consent'}
          </p>

          <div className="mode-toggle-login">
            <button className={mode === 'simple' ? 'active' : ''} onClick={() => switchMode('simple')}>
              For Everyone
            </button>
            <button className={mode === 'expert' ? 'active' : ''} onClick={() => switchMode('expert')}>
              Developer
            </button>
          </div>

          <form onSubmit={handleLogin}>
            <input
              type="text"
              placeholder="Access key  (hsip_…)"
              value={apiKey}
              onChange={e => setApiKey(e.target.value)}
              autoFocus
            />
            <button type="submit">
              {mode === 'simple' ? 'Sign in' : 'Connect'}
            </button>
          </form>
          {error && <p className="error">{error}</p>}

          <div className="login-key-hint">
            <p className="login-hint-head">
              First time? Your key was shown in the terminal and saved to:
            </p>
            <div className="login-hint-paths">
              <span className="login-hint-os">Windows</span>
              <code>%APPDATA%\HSIP\admin.key</code>
              <span className="login-hint-os">Mac / Linux</span>
              <code>~/.hsip/admin.key</code>
            </div>
          </div>
        </div>
      </div>
    );
  }

  const tabs = mode === 'simple' ? SIMPLE_TABS : EXPERT_TABS;
  const isFullscreen = tab === 'messages';

  return (
    <div className="app">
      {onboarding && <Onboarding onComplete={() => setOnboarding(false)} />}

      {/* ── Sidebar navigation ── */}
      <aside className="app-nav">
        <div className="app-nav-logo">
          <span className="app-logo-icon">🔐</span>
          <span className="app-logo-text">HSIP</span>
          {mode === 'expert' && <span className="mode-badge">Dev</span>}
        </div>

        <nav className="app-nav-items">
          {tabs.map(t => (
            <button
              key={t.id}
              className={`nav-item${tab === t.id ? ' nav-item--active' : ''}`}
              onClick={() => setTab(t.id)}
            >
              <span className="nav-icon">{t.icon}</span>
              <span className="nav-label">{t.label}</span>
            </button>
          ))}
        </nav>

        <div className="app-nav-footer">
          {mode === 'simple' && (
            <button className="nav-footer-btn" onClick={() => setOnboarding(true)}>
              <span>❓</span> What is HSIP?
            </button>
          )}
          <button
            className="nav-footer-btn"
            onClick={() => switchMode(mode === 'simple' ? 'expert' : 'simple')}
          >
            <span>{mode === 'simple' ? '⚙️' : '👤'}</span>
            {mode === 'simple' ? 'Dev Mode' : 'Simple Mode'}
          </button>
          <button className="nav-footer-btn nav-footer-btn--logout" onClick={logout}>
            <span>↩</span> Sign out
          </button>
        </div>
      </aside>

      {/* ── Page content ── */}
      <div className="app-content">
        <main className={isFullscreen ? 'main--fullscreen' : ''}>
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
    </div>
  );
}
