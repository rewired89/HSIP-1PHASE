import React, { useState } from 'react';
import { request } from './api';
import Identity         from './pages/Identity';
import Consent          from './pages/Consent';
import Messages         from './pages/Messages';
import Audit            from './pages/Audit';
import Keys             from './pages/Keys';
import Credentials      from './pages/Credentials';
import HomeDashboard    from './pages/HomeDashboard';
import ProveIt          from './pages/ProveIt';
import ConsentWallet    from './pages/ConsentWallet';
import AIWatch          from './pages/AIWatch';
import TrackerInspector from './pages/TrackerInspector';
import ProtectionSetup  from './pages/ProtectionSetup';
import NetworkMonitor   from './pages/NetworkMonitor';
import Onboarding       from './pages/Onboarding';
import './App.css';

const PRIMARY_TABS = [
  { id: 'home',              label: '🏠 Home' },
  { id: 'messages',          label: '💬 Messages' },
  { id: 'network',           label: '🌐 Traffic Monitor' },
  { id: 'prove-it',          label: '✍️ Alibi' },
  { id: 'consent-wallet',    label: '🛡️ My Consents' },
  { id: 'ai-watch',          label: '🤖 AI Watch' },
  { id: 'tracker-inspector', label: '🔍 Trackers' },
  { id: 'protection',        label: '🔒 Protection' },
];

const ADVANCED_TABS = [
  { id: 'identity',    label: 'Identity' },
  { id: 'consent',     label: 'Consent' },
  { id: 'credentials', label: 'Credentials' },
  { id: 'audit',       label: 'Audit' },
  { id: 'keys',        label: 'Keys' },
];

export default function App() {
  const [apiKey,     setApiKey]     = useState(localStorage.getItem('hsip_api_key') || '');
  const [authed,     setAuthed]     = useState(false);
  const [error,      setError]      = useState('');
  const [tab,        setTab]        = useState('home');
  const [showAdv,    setShowAdv]    = useState(false);
  const [onboarding, setOnboarding] = useState(false);

  const isAdvancedTab = ADVANCED_TABS.some(t => t.id === tab);

  async function handleLogin(e) {
    e.preventDefault();
    try {
      await request('POST', '/v1/identity', null, apiKey);
      localStorage.setItem('hsip_api_key', apiKey);
      setAuthed(true);
      setError('');
      if (!localStorage.getItem('hsip_onboarding_done')) {
        setOnboarding(true);
      }
    } catch {
      setError('Invalid access key. Please check and try again.');
    }
  }

  function logout() {
    localStorage.removeItem('hsip_api_key');
    setAuthed(false);
  }

  function navigateTo(id) {
    if (ADVANCED_TABS.some(t => t.id === id)) setShowAdv(true);
    setTab(id);
  }

  if (!authed) {
    return (
      <div className="login-screen">
        <div className="login-card">
          <div className="login-logo">🔐</div>
          <h1>HSIP</h1>
          <p>Your personal privacy and identity hub</p>
          <form onSubmit={handleLogin}>
            <input
              type="text"
              placeholder="Enter your access key (hsip_…)"
              value={apiKey}
              onChange={e => setApiKey(e.target.value)}
            />
            <button type="submit">Enter</button>
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
      {onboarding && (
        <Onboarding onComplete={() => setOnboarding(false)} />
      )}

      <header>
        <h1 className="app-title">HSIP</h1>
        <nav>
          {PRIMARY_TABS.map(t => (
            <button
              key={t.id}
              className={tab === t.id ? 'active' : ''}
              onClick={() => navigateTo(t.id)}
            >
              {t.label}
            </button>
          ))}

          <button
            className={`adv-toggle${showAdv ? ' adv-toggle--open' : ''}${isAdvancedTab ? ' active' : ''}`}
            onClick={() => setShowAdv(v => !v)}
            title="Advanced developer tools: Identity, Credentials, Audit, Keys"
          >
            Advanced {showAdv ? '▴' : '▾'}
          </button>

          {showAdv && ADVANCED_TABS.map(t => (
            <button
              key={t.id}
              className={`adv-tab${tab === t.id ? ' active' : ''}`}
              onClick={() => setTab(t.id)}
            >
              {t.label}
            </button>
          ))}

          <div className="nav-right">
            <button
              className="mode-switch-btn ob-replay-btn"
              onClick={() => setOnboarding(true)}
              title="What does HSIP do on my machine?"
            >
              ❓ What is HSIP?
            </button>
            <button onClick={logout}>Logout</button>
          </div>
        </nav>
      </header>

      <main>
        {tab === 'home'              && <HomeDashboard    onNavigate={navigateTo} />}
        {tab === 'messages'          && <Messages         apiKey={apiKey} />}
        {tab === 'network'           && <NetworkMonitor   apiKey={apiKey} />}
        {tab === 'prove-it'          && <ProveIt          apiKey={apiKey} />}
        {tab === 'consent-wallet'    && <ConsentWallet    apiKey={apiKey} />}
        {tab === 'ai-watch'          && <AIWatch          apiKey={apiKey} />}
        {tab === 'tracker-inspector' && <TrackerInspector />}
        {tab === 'protection'        && <ProtectionSetup  apiKey={apiKey} />}
        {tab === 'identity'          && <Identity         apiKey={apiKey} />}
        {tab === 'consent'           && <Consent          apiKey={apiKey} />}
        {tab === 'credentials'       && <Credentials      apiKey={apiKey} />}
        {tab === 'audit'             && <Audit            apiKey={apiKey} />}
        {tab === 'keys'              && <Keys             apiKey={apiKey} />}
      </main>
    </div>
  );
}
