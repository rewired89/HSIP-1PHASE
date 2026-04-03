import React, { useState, useEffect, useRef } from 'react';
import { request } from '../api';

// ── Category colours ──────────────────────────────────────────────────────────
const CAT_COLOR = {
  advertising:    { bg: '#2d1212', text: '#fc8181', label: '📢 Advertising' },
  analytics:      { bg: '#12192d', text: '#90cdf4', label: '📊 Analytics' },
  social:         { bg: '#1a1228', text: '#d6bcfa', label: '👁 Social' },
  fingerprinting: { bg: '#1f1a0d', text: '#f6e05e', label: '🔍 Fingerprinting' },
  telemetry:      { bg: '#0d1f12', text: '#68d391', label: '📡 Telemetry' },
};
function catStyle(cat) {
  return CAT_COLOR[cat] || { bg: '#1a1d27', text: '#a0aec0', label: cat || 'unknown' };
}

// ── Setup wizard ──────────────────────────────────────────────────────────────
function SetupWizard({ port, onDone }) {
  const [tab, setTab] = useState('windows');
  const [copied, setCopied] = useState(false);
  function copy(text) {
    navigator.clipboard.writeText(text);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  const steps = {
    windows: [
      { n: 1, title: 'Open Proxy Settings',
        desc: 'Press Win + I → Network & Internet → Proxy' },
      { n: 2, title: 'Enable Manual Proxy',
        desc: 'Under "Manual proxy setup" turn the toggle ON' },
      { n: 3, title: 'Enter the address',
        desc: `Address: 127.0.0.1   Port: ${port}` },
      { n: 4, title: 'Save',
        desc: 'Click Save. Open any website — HSIP now sees all traffic.' },
    ],
    mac: [
      { n: 1, title: 'Open Network Settings',
        desc: 'System Settings → Network → select Wi-Fi or Ethernet → Details' },
      { n: 2, title: 'Go to Proxies tab',
        desc: 'Click the Proxies tab in the connection details window' },
      { n: 3, title: 'Enable HTTP + HTTPS proxy',
        desc: `Turn ON Web Proxy (HTTP) and Secure Web Proxy (HTTPS). Server: 127.0.0.1  Port: ${port}` },
      { n: 4, title: 'Apply',
        desc: 'Click OK then Apply. HSIP now intercepts all browser traffic.' },
    ],
    firefox: [
      { n: 1, title: 'Open Firefox Settings',
        desc: 'Hamburger menu → Settings → search "proxy"' },
      { n: 2, title: 'Manual proxy configuration',
        desc: 'Select "Manual proxy configuration"' },
      { n: 3, title: 'Enter proxy address',
        desc: `HTTP Proxy: 127.0.0.1   Port: ${port}` },
      { n: 4, title: 'Also use for HTTPS',
        desc: 'Check "Also use this proxy for HTTPS". Click OK.' },
    ],
  };

  return (
    <div className="tm-wizard card">
      <div className="tm-wizard-header">
        <div className="tm-wizard-icon">🛡</div>
        <div>
          <h3 className="tm-wizard-title">Set up Traffic Monitor</h3>
          <p className="tm-wizard-sub">
            Point your browser's proxy to <strong>127.0.0.1:{port}</strong> and HSIP will
            show you — and block — every tracker that tries to follow you.
          </p>
        </div>
      </div>

      <div className="tm-wizard-tabs">
        {[['windows','🪟 Windows'],['mac','🍎 Mac'],['firefox','🦊 Firefox only']].map(([id,label]) => (
          <button key={id}
            className={`tm-tab${tab === id ? ' tm-tab--active' : ''}`}
            onClick={() => setTab(id)}>{label}</button>
        ))}
      </div>

      <div className="tm-wizard-steps">
        {steps[tab].map(s => (
          <div key={s.n} className="tm-step">
            <div className="tm-step-num">{s.n}</div>
            <div>
              <div className="tm-step-title">{s.title}</div>
              <div className="tm-step-desc">{s.desc}</div>
            </div>
          </div>
        ))}
      </div>

      <div className="tm-wizard-foot">
        <div className="tm-proxy-pill">
          <span>Proxy address</span>
          <code>127.0.0.1:{port}</code>
          <button onClick={() => copy(`127.0.0.1:${port}`)}>
            {copied ? '✓ Copied' : 'Copy'}
          </button>
        </div>
        <button className="primary tm-done-btn" onClick={onDone}>
          I've set it up — show me my traffic →
        </button>
      </div>
    </div>
  );
}

// ── Stats bar ─────────────────────────────────────────────────────────────────
function StatsBar({ events }) {
  const total   = events.length;
  const blocked = events.filter(e => e.verdict === 'blocked').length;
  const allowed = total - blocked;
  const pct     = total ? Math.round((blocked / total) * 100) : 0;

  const cats = {};
  events.filter(e => e.verdict === 'blocked').forEach(e => {
    const c = e.category || 'unknown';
    cats[c] = (cats[c] || 0) + 1;
  });
  const topCat = Object.entries(cats).sort((a, b) => b[1] - a[1])[0];

  return (
    <div className="tm-stats">
      <div className="tm-stat tm-stat--blocked">
        <div className="tm-stat-num">{blocked}</div>
        <div className="tm-stat-label">Blocked</div>
      </div>
      <div className="tm-stat tm-stat--allowed">
        <div className="tm-stat-num">{allowed}</div>
        <div className="tm-stat-label">Allowed</div>
      </div>
      <div className="tm-stat">
        <div className="tm-stat-num">{pct}%</div>
        <div className="tm-stat-label">Tracker rate</div>
      </div>
      {topCat && (
        <div className="tm-stat">
          <div className="tm-stat-num"
            style={{ fontSize: '0.8rem', color: catStyle(topCat[0]).text }}>
            {catStyle(topCat[0]).label}
          </div>
          <div className="tm-stat-label">Most blocked type</div>
        </div>
      )}
      <div className="tm-stat tm-stat--live">
        <div className="tm-pulse-dot" />
        <div className="tm-stat-label">Live</div>
      </div>
    </div>
  );
}

// ── Event row ─────────────────────────────────────────────────────────────────
function EventRow({ ev }) {
  const blocked = ev.verdict === 'blocked';
  const cs  = catStyle(ev.category);
  const time = new Date(ev.ts_ms).toLocaleTimeString([], {
    hour: '2-digit', minute: '2-digit', second: '2-digit',
  });

  return (
    <div className={`tm-row ${blocked ? 'tm-row--blocked' : 'tm-row--allowed'}`}>
      <div className="tm-row-verdict">
        {blocked ? '🚫 BLOCKED' : '✓ allowed'}
      </div>
      <div className="tm-row-host">{ev.host || '—'}</div>
      {ev.category
        ? <div className="tm-row-cat" style={{ background: cs.bg, color: cs.text }}>{cs.label}</div>
        : <div className="tm-row-cat" />}
      <div className="tm-row-method">{ev.method}</div>
      <div className="tm-row-time">{time}</div>
    </div>
  );
}

// ── Top blocked ───────────────────────────────────────────────────────────────
function TopBlocked({ events }) {
  const counts = {};
  events.filter(e => e.verdict === 'blocked').forEach(e => {
    if (!counts[e.host]) counts[e.host] = { host: e.host, category: e.category, n: 0 };
    counts[e.host].n++;
  });
  const rows = Object.values(counts).sort((a, b) => b.n - a.n).slice(0, 10);
  if (!rows.length) return null;

  return (
    <div className="card" style={{ marginTop: '1rem' }}>
      <h4 className="tm-section-title">Top blocked this session</h4>
      <div className="tm-top-list">
        {rows.map((r, i) => {
          const cs = catStyle(r.category);
          return (
            <div key={r.host} className="tm-top-row">
              <span className="tm-top-rank">#{i + 1}</span>
              <span className="tm-top-host">{r.host}</span>
              <span className="tm-top-cat" style={{ color: cs.text }}>{cs.label}</span>
              <span className="tm-top-count">{r.n}×</span>
            </div>
          );
        })}
      </div>
    </div>
  );
}

// ── Main ──────────────────────────────────────────────────────────────────────
export default function NetworkMonitor({ apiKey }) {
  const [proxyEnabled, setProxyEnabled] = useState(false);
  const [port,         setPort]         = useState(8877);
  const [events,       setEvents]       = useState([]);
  const [showSetup,    setShowSetup]    = useState(false);
  const [toggling,     setToggling]     = useState(false);
  const [filter,       setFilter]       = useState('all');
  const [search,       setSearch]       = useState('');
  const feedRef = useRef(null);

  useEffect(() => {
    request('GET', '/v1/proxy/status', null, apiKey)
      .then(s => {
        setProxyEnabled(s.enabled);
        setPort(s.port || 8877);
        if (!s.enabled) setShowSetup(true);
      })
      .catch(() => setShowSetup(true));
  }, []);

  useEffect(() => {
    if (!proxyEnabled) return;
    const id = setInterval(() => {
      request('GET', '/v1/proxy/log', null, apiKey)
        .then(list => {
          if (Array.isArray(list)) setEvents(list.slice().reverse());
        })
        .catch(() => {});
    }, 1500);
    return () => clearInterval(id);
  }, [proxyEnabled, apiKey]);

  async function toggleProxy() {
    setToggling(true);
    try {
      if (proxyEnabled) {
        await request('POST', '/v1/proxy/disable', null, apiKey);
        setProxyEnabled(false);
        setEvents([]);
      } else {
        const s = await request('POST', '/v1/proxy/enable', null, apiKey);
        setProxyEnabled(true);
        setPort(s.port || 8877);
        setShowSetup(false);
      }
    } catch (e) { alert(e.message); }
    setToggling(false);
  }

  const filtered = events.filter(ev => {
    if (filter === 'blocked' && ev.verdict !== 'blocked') return false;
    if (filter === 'allowed' && ev.verdict !== 'allowed') return false;
    if (search && !ev.host.toLowerCase().includes(search.toLowerCase())) return false;
    return true;
  });

  return (
    <div>
      {/* Header */}
      <div className="tm-header">
        <div>
          <h2 className="tm-title">🌐 Traffic Monitor</h2>
          <p className="tm-subtitle">
            {proxyEnabled
              ? `Intercepting all HTTP/HTTPS traffic on port ${port}. Trackers blocked automatically.`
              : 'Proxy off — enable to monitor and block trackers in real time.'}
          </p>
        </div>
        <div className="tm-header-actions">
          {proxyEnabled && (
            <button className="tm-setup-link" onClick={() => setShowSetup(v => !v)}>
              {showSetup ? 'Hide setup' : '⚙ Setup guide'}
            </button>
          )}
          <button
            className={`tm-toggle ${proxyEnabled ? 'tm-toggle--on' : 'tm-toggle--off'}`}
            onClick={toggleProxy} disabled={toggling}>
            {toggling ? '…' : proxyEnabled ? '🟢 Proxy ON' : '⚫ Enable Proxy'}
          </button>
        </div>
      </div>

      {/* Setup wizard */}
      {showSetup && (
        <SetupWizard port={port} onDone={() => {
          setShowSetup(false);
          if (!proxyEnabled) toggleProxy();
        }} />
      )}

      {/* Off state */}
      {!proxyEnabled && !showSetup && (
        <div className="card tm-disabled-state">
          <div style={{ fontSize: '3rem', marginBottom: '1rem' }}>🔒</div>
          <h3>Traffic Monitor is off</h3>
          <p>
            When enabled, HSIP sits between your browser and the internet.
            Every request passes through — tracking companies get blocked before they load,
            and you see exactly what your apps are sending in real time.
          </p>
          <div style={{ display: 'flex', gap: '0.75rem', marginTop: '1.25rem', justifyContent: 'center', flexWrap: 'wrap' }}>
            <button className="primary" onClick={toggleProxy} disabled={toggling}>
              {toggling ? 'Enabling…' : '🛡 Enable Traffic Monitor'}
            </button>
            <button className="consumer-reset-btn" onClick={() => setShowSetup(true)}>
              How does it work?
            </button>
          </div>
        </div>
      )}

      {/* Live dashboard */}
      {proxyEnabled && (
        <>
          <StatsBar events={events} />

          <div className="tm-filters">
            <div className="tm-filter-tabs">
              {[['all','All traffic'],['blocked','Blocked'],['allowed','Allowed']].map(([v,l]) => (
                <button key={v}
                  className={`tm-filter-tab${filter === v ? ' tm-filter-tab--active' : ''}`}
                  onClick={() => setFilter(v)}>{l}</button>
              ))}
            </div>
            <input className="tm-search" placeholder="Filter by domain…"
              value={search} onChange={e => setSearch(e.target.value)} />
          </div>

          <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
            <div className="tm-feed-head">
              <span>Verdict</span><span>Domain</span>
              <span>Category</span><span>Method</span><span>Time</span>
            </div>
            <div className="tm-feed" ref={feedRef}>
              {filtered.length === 0 && (
                <div className="tm-feed-empty">
                  {events.length === 0
                    ? 'No traffic yet — open any website in your browser to see requests appear here.'
                    : 'No events match your filter.'}
                </div>
              )}
              {filtered.map(ev => <EventRow key={ev.id} ev={ev} />)}
            </div>
          </div>

          <TopBlocked events={events} />
        </>
      )}
    </div>
  );
}
