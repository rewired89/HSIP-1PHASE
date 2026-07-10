import React, { useState } from 'react';
import { request } from './api';
import Identity        from './pages/Identity';
import Consent         from './pages/Consent';
import Messages        from './pages/Messages';
import Audit           from './pages/Audit';
import Keys            from './pages/Keys';
import Credentials     from './pages/Credentials';
import Decisions       from './pages/Decisions';
import HomeDashboard   from './pages/HomeDashboard';
import ProveIt         from './pages/ProveIt';
import ConsentWallet   from './pages/ConsentWallet';
import AIWatch         from './pages/AIWatch';
import DecisionsSimple from './pages/DecisionsSimple';
import TrackerInspector  from './pages/TrackerInspector';
import ProtectionSetup   from './pages/ProtectionSetup';
import NetworkMonitor    from './pages/NetworkMonitor';
import Onboarding        from './pages/Onboarding';
import FinanceDashboard  from './pages/FinanceDashboard';
import './App.css';

const SIMPLE_TABS = [
  { id: 'home',              icon: '🏠', label: 'Home',       subtitle: 'Your security overview' },
  { id: 'finance',           icon: '🏦', label: 'Finance',    subtitle: 'Verified records for payments' },
  { id: 'messages',          icon: '💬', label: 'Messages',   subtitle: 'Tamper-proof signed messages' },
  { id: 'network',           icon: '🌐', label: 'Traffic',    subtitle: 'See what your device connects to' },
  { id: 'prove-it',          icon: '✍️',  label: 'Alibi',      subtitle: 'Prove something happened when' },
  { id: 'consent-wallet',    icon: '🛡️',  label: 'Consents',   subtitle: 'Control who acts on your behalf' },
  { id: 'ai-watch',          icon: '🤖', label: 'AI Watch',   subtitle: 'Monitor AI agents in real time' },
  { id: 'decisions-simple',  icon: '📈', label: 'AI Decisions', subtitle: 'See what your trading bots decided' },
  { id: 'tracker-inspector', icon: '🔍', label: 'Trackers',   subtitle: 'See who is tracking you online' },
  { id: 'protection',        icon: '🔒', label: 'Protection', subtitle: 'Block ads, trackers & malware' },
];

const EXPERT_TABS = [
  { id: 'identity',    icon: '🆔', label: 'Identity',    subtitle: 'Your Ed25519 keypair' },
  { id: 'consent',     icon: '✅', label: 'Consent',     subtitle: 'Grant & revoke peer access' },
  { id: 'messages',    icon: '💬', label: 'Messages',    subtitle: 'Sign & verify signatures' },
  { id: 'credentials', icon: '🏅', label: 'Credentials', subtitle: 'Issue verifiable credentials' },
  { id: 'decisions',   icon: '📈', label: 'Decisions',   subtitle: 'Audit AI-agent trading decisions' },
  { id: 'audit',       icon: '📋', label: 'Audit',       subtitle: 'Append-only action log' },
  { id: 'keys',        icon: '🔑', label: 'Keys',        subtitle: 'Manage API keys' },
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
  const [provisioning, setProvisioning] = useState(false);

  function switchMode(m) {
    setMode(m);
    localStorage.setItem('hsip_mode', m);
    setTab(m === 'simple' ? 'home' : 'identity');
  }

  async function handleLogin(e) {
    e.preventDefault();
    const trimmedKey = apiKey.trim();
    try {
      await request('POST', '/v1/identity', null, trimmedKey);
      localStorage.setItem('hsip_api_key', trimmedKey);
      setApiKey(trimmedKey);
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

  async function handleGetTrialKey() {
    setProvisioning(true);
    setError('');
    try {
      const res = await fetch('/v1/sandbox/provision', { method: 'POST' });
      const data = await res.json();
      if (!res.ok) throw new Error(data.error || 'Provision failed');
      const key = data.api_key;
      // verify the key works then auto-sign-in
      await request('POST', '/v1/identity', null, key);
      localStorage.setItem('hsip_api_key', key);
      setApiKey(key);
      setAuthed(true);
      setError('');
      setTab('home');
    } catch (err) {
      setError(err.message || 'Could not get a trial key. Try again.');
    } finally {
      setProvisioning(false);
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

          <button
            className="trial-key-btn"
            onClick={handleGetTrialKey}
            disabled={provisioning}
          >
            {provisioning ? 'Getting your key…' : '✨ Try it free — get a 24-hour key'}
          </button>

          <div className="login-divider"><span>or sign in with your own key</span></div>

          <form onSubmit={handleLogin}>
            <input
              type="text"
              placeholder="Access key  (hsip_…)"
              value={apiKey}
              onChange={e => setApiKey(e.target.value)}
            />
            <button type="submit">
              {mode === 'simple' ? 'Sign in' : 'Connect'}
            </button>
          </form>
          {error && <p className="error">{error}</p>}

          <div className="login-key-hint">
            <p className="login-hint-head">
              Running HSIP locally? Your key is saved to:
            </p>
            <div className="login-hint-paths">
              <span className="login-hint-os">Windows</span>
              <code>%APPDATA%\HSIP\admin.key</code>
              <span className="login-hint-os">Mac / Linux</span>
              <code>~/.hsip/admin.key</code>
            </div>
          </div>
          <div className="login-features">
            {['🔒 Local-only', '✍️ Ed25519 signing', '🤖 AI governance', '🏦 Finance-ready'].map(f => (
              <span key={f} className="login-feature">{f}</span>
            ))}
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
              <span className="nav-text">
                <span className="nav-label">{t.label}</span>
                {t.subtitle && <span className="nav-sub">{t.subtitle}</span>}
              </span>
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
              {tab === 'finance'           && <FinanceDashboard apiKey={apiKey} />}
              {tab === 'messages'          && <Messages         apiKey={apiKey} />}
              {tab === 'network'           && <NetworkMonitor   apiKey={apiKey} />}
              {tab === 'prove-it'          && <ProveIt          apiKey={apiKey} />}
              {tab === 'consent-wallet'    && <ConsentWallet    apiKey={apiKey} />}
              {tab === 'ai-watch'          && <AIWatch          apiKey={apiKey} />}
              {tab === 'decisions-simple'  && <DecisionsSimple  apiKey={apiKey} />}
              {tab === 'tracker-inspector' && <TrackerInspector />}
              {tab === 'protection'        && <ProtectionSetup  apiKey={apiKey} />}
            </>
          ) : (
            <>
              {tab === 'identity'    && <Identity    apiKey={apiKey} />}
              {tab === 'consent'     && <Consent     apiKey={apiKey} />}
              {tab === 'messages'    && <Messages    apiKey={apiKey} />}
              {tab === 'credentials' && <Credentials apiKey={apiKey} />}
              {tab === 'decisions'   && <Decisions   apiKey={apiKey} />}
              {tab === 'audit'       && <Audit       apiKey={apiKey} />}
              {tab === 'keys'        && <Keys        apiKey={apiKey} />}
            </>
          )}
        </main>
      </div>
    </div>
  );
}
