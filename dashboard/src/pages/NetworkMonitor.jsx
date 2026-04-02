import React, { useState, useEffect, useRef } from 'react';
import { request } from '../api';

function timeSince(ms) {
  const s = Math.floor((Date.now() - ms) / 1000);
  if (s < 5)  return 'just now';
  if (s < 60) return `${s}s ago`;
  return `${Math.floor(s / 60)}m ago`;
}

const CATEGORY_COLOR = {
  'Analytics':        '#f6ad55',
  'Advertising':      '#fc8181',
  'Session Recording':'#e53e3e',
  'Social':           '#b794f4',
  'Marketing':        '#f6e05e',
  'CDN':              '#68d391',
};

export default function NetworkMonitor({ apiKey }) {
  const [status,   setStatus]   = useState(null);
  const [entries,  setEntries]  = useState([]);
  const [toggling, setToggling] = useState(false);
  const [dnsError, setDnsError] = useState('');
  const liveRef = useRef(null);

  useEffect(() => {
    load();
    const id = setInterval(load, 2000);
    return () => clearInterval(id);
  }, []);

  async function load() {
    try {
      const s = await request('GET', '/v1/dns/status', null, apiKey);
      setStatus(s);
      if (s.running) {
        const log = await request('GET', '/v1/dns/log', null, apiKey);
        setEntries(prev => {
          const incoming = log.entries || [];
          if (incoming.length === 0) return prev;
          // Merge with existing, keep newest 100
          const ids = new Set(prev.map(e => e.domain + e.timestamp_ms));
          const fresh = incoming.filter(e => !ids.has(e.domain + e.timestamp_ms));
          return [...fresh, ...prev].slice(0, 100);
        });
      }
    } catch {}
  }

  async function toggleDns() {
    if (!status) return;
    setToggling(true);
    setDnsError('');
    try {
      if (status.running) {
        await request('POST', '/v1/dns/disable', null, apiKey);
        setEntries([]);
      } else {
        await request('POST', '/v1/dns/enable', { port: 5300 }, apiKey);
      }
      await load();
    } catch (e) { setDnsError(e.message); }
    setToggling(false);
  }

  const blocked = entries.filter(e => e.blocked);
  const allowed = entries.filter(e => !e.blocked);

  if (!status) {
    return <div className="card"><p className="empty">Connecting to HSIP…</p></div>;
  }

  return (
    <div>
      <div className="consumer-hero">
        <div className="consumer-hero-icon">🌐</div>
        <h2>Network Monitor</h2>
        <p>
          See every tracker your computer tries to reach — and watch HSIP block them in real time.
          This shows what companies are trying to collect from you right now.
        </p>
      </div>

      {/* DNS toggle */}
      <div className={`card dns-card${status.running ? ' dns-card--active' : ''}`}>
        <div className="dns-header">
          <div className="dns-header-left">
            <div className="dns-icon">{status.running ? '🟢' : '⚪'}</div>
            <div>
              <h2 className="dns-title">
                {status.running ? 'Protection is ON' : 'Protection is OFF'}
              </h2>
              <p className="dns-subtitle">
                {status.running
                  ? `Blocking ${status.blocklist_size} tracker domains on every app on your computer`
                  : 'Turn on to start blocking trackers and see live traffic'}
              </p>
            </div>
          </div>
          <button
            className={`dns-toggle-btn${status.running ? ' dns-toggle-btn--on' : ''}`}
            onClick={toggleDns} disabled={toggling}>
            {toggling ? '…' : status.running ? 'Turn Off' : 'Turn On'}
          </button>
        </div>

        {dnsError && <div className="dns-error-banner">⚠️ {dnsError}</div>}

        {status.running && (
          <div className="dns-stats-row">
            <div className="dns-stat">
              <span className="dns-stat-num">{status.blocked_total.toLocaleString()}</span>
              <span className="dns-stat-label">blocked since start</span>
            </div>
            <div className="dns-stat">
              <span className="dns-stat-num">{status.queries_total.toLocaleString()}</span>
              <span className="dns-stat-label">total DNS lookups</span>
            </div>
            <div className="dns-stat">
              <span className="dns-stat-num">{status.blocklist_size}</span>
              <span className="dns-stat-label">tracker domains</span>
            </div>
            <div className="dns-stat">
              <span className="dns-stat-num" style={{ color: '#fc8181' }}>{blocked.length}</span>
              <span className="dns-stat-label">blocked in feed</span>
            </div>
          </div>
        )}
      </div>

      {!status.running && (
        <div className="card" style={{ textAlign: 'center', padding: '3rem' }}>
          <div style={{ fontSize: '3rem', marginBottom: '1rem' }}>👁️</div>
          <h3 style={{ marginBottom: '0.5rem' }}>Turn on protection to see what's watching you</h3>
          <p style={{ color: '#718096', marginBottom: '1.5rem' }}>
            When active, every tracker your computer contacts — from any app, any browser —
            appears here in real time.
          </p>
          <button className="primary" onClick={toggleDns} disabled={toggling}>
            {toggling ? '…' : 'Turn On Protection'}
          </button>
        </div>
      )}

      {status.running && (
        <div className="card" style={{ padding: 0, overflow: 'hidden' }}>
          <div style={{ padding: '1rem 1.5rem 0', display: 'flex', justifyContent: 'space-between', alignItems: 'center' }}>
            <h2 style={{ margin: 0 }}>Live Traffic</h2>
            <span style={{ fontSize: '0.75rem', color: '#718096' }}>Updates every 2 seconds</span>
          </div>

          {entries.length === 0 && (
            <div style={{ padding: '2rem', textAlign: 'center', color: '#718096' }}>
              <p>No traffic yet — start browsing and tracker attempts will appear here.</p>
            </div>
          )}

          <div className="live-feed" ref={liveRef}>
            {entries.map((e, i) => (
              <div key={i} className={`feed-row ${e.blocked ? 'feed-row--blocked' : 'feed-row--allowed'}`}>
                <span className="feed-verdict">{e.blocked ? '🚫 BLOCKED' : '✓'}</span>
                <span className="feed-domain">{e.domain}</span>
                {e.vendor && <span className="feed-vendor">{e.vendor}</span>}
                {e.category && (
                  <span className="feed-category"
                    style={{ background: CATEGORY_COLOR[e.category] || '#2d3748', color: '#0f1117' }}>
                    {e.category}
                  </span>
                )}
                <span className="feed-time">{timeSince(e.timestamp_ms)}</span>
              </div>
            ))}
          </div>
        </div>
      )}

      {status.running && blocked.length > 0 && (
        <div className="card">
          <h2>Top blocked this session</h2>
          <div className="blocked-list">
            {Object.entries(
              blocked.reduce((acc, e) => {
                const k = e.vendor || e.domain;
                acc[k] = (acc[k] || 0) + 1;
                return acc;
              }, {})
            )
              .sort((a, b) => b[1] - a[1])
              .slice(0, 10)
              .map(([name, count], i) => (
                <div key={i} className="blocked-item">
                  <span className="blocked-rank">#{i + 1}</span>
                  <span className="blocked-vendor" style={{ flex: 1 }}>{name}</span>
                  <span style={{ color: '#fc8181', fontWeight: 700 }}>{count}×</span>
                </div>
              ))}
          </div>
        </div>
      )}
    </div>
  );
}
