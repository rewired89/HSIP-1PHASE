import React, { useState, useMemo } from 'react';
import { TRACKERS, CATEGORIES, RISK_LEVEL, TRACKER_STATS } from '../data/trackers';

function RiskBadge({ risk }) {
  const r = RISK_LEVEL[risk];
  return (
    <span className="risk-badge" style={{ background: r.bg, color: r.color }}>
      {r.label}
    </span>
  );
}

function TrackerCard({ tracker }) {
  const [expanded, setExpanded] = useState(false);
  return (
    <div className={`tracker-card tracker-card--${tracker.risk}`}>
      <div className="tracker-card-header" onClick={() => setExpanded(v => !v)}>
        <div className="tracker-card-left">
          <div className="tracker-vendor">{tracker.vendor}</div>
          <div className="tracker-plain">{tracker.plain}</div>
          <div className="tracker-domain">{tracker.domain}</div>
        </div>
        <div className="tracker-card-right">
          <RiskBadge risk={tracker.risk} />
          {tracker.safeToBlock
            ? <span className="block-badge block-badge--yes">✓ Safe to block</span>
            : <span className="block-badge block-badge--no">⚠ Use caution</span>}
          <span className="tracker-expand">{expanded ? '▲' : '▼'}</span>
        </div>
      </div>
      {expanded && (
        <div className="tracker-card-body">
          <p>{tracker.description}</p>
          <div className="tracker-category">Category: {tracker.category}</div>
        </div>
      )}
    </div>
  );
}

function LookupResult({ result }) {
  if (!result) return null;
  if (result === 'clean') {
    return (
      <div className="lookup-result lookup-result--clean">
        <span>✅</span>
        <div>
          <strong>Not a known tracker service</strong>
          <p>This domain isn't in HSIP's database of third-party tracking services. That means it's probably not secretly embedded in other websites to spy on you — but it doesn't mean the site itself doesn't collect your data. Every website collects some data about visitors.</p>
        </div>
      </div>
    );
  }
  return (
    <div className={`lookup-result lookup-result--found lookup-result--${result.risk}`}>
      <span style={{ fontSize: '1.5rem' }}>
        {result.risk === 'critical' || result.risk === 'high' ? '🚨' : '⚠️'}
      </span>
      <div>
        <strong>{result.vendor}</strong>
        <div className="lookup-plain">{result.plain}</div>
        <p style={{ marginTop: '0.5rem' }}>{result.description}</p>
        <div style={{ display: 'flex', gap: '0.5rem', marginTop: '0.5rem', flexWrap: 'wrap' }}>
          <RiskBadge risk={result.risk} />
          {result.safeToBlock
            ? <span className="block-badge block-badge--yes">✓ Safe to block</span>
            : <span className="block-badge block-badge--no">⚠ Use caution before blocking</span>}
        </div>
      </div>
    </div>
  );
}

export default function TrackerInspector() {
  const [search,   setSearch]   = useState('');
  const [category, setCategory] = useState('All');
  const [riskFilter, setRiskFilter] = useState('All');
  const [lookup,   setLookup]   = useState('');
  const [lookupResult, setLookupResult] = useState(null);

  const filtered = useMemo(() => {
    let list = TRACKERS;
    if (category !== 'All') list = list.filter(t => t.category === category);
    if (riskFilter !== 'All') list = list.filter(t => t.risk === riskFilter.toLowerCase());
    if (search.trim()) {
      const q = search.toLowerCase();
      list = list.filter(t =>
        t.vendor.toLowerCase().includes(q) ||
        t.domain.toLowerCase().includes(q) ||
        t.plain.toLowerCase().includes(q)
      );
    }
    return list;
  }, [search, category, riskFilter]);

  function doLookup() {
    if (!lookup.trim()) return;
    const host = lookup.trim().toLowerCase()
      .replace(/^https?:\/\//, '')
      .replace(/\/.*$/, '');
    const match = TRACKERS.find(t => {
      const pattern = t.domain.replace('*.', '');
      return host === pattern || host.endsWith('.' + pattern) || host.endsWith(pattern);
    });
    setLookupResult(match || 'clean');
  }

  return (
    <div>
      <div className="consumer-hero">
        <div className="consumer-hero-icon">🔍</div>
        <h2>Tracker Inspector</h2>
        <p>
          HSIP knows about {TRACKER_STATS.total} tracking companies. Browse what they do,
          check any domain, and understand exactly what's following you across the web.
        </p>
      </div>

      {/* Stats bar */}
      <div className="tracker-stats card">
        <div className="tracker-stat">
          <span className="tracker-stat-num" style={{ color: '#fc8181' }}>
            {TRACKER_STATS.critical}
          </span>
          <span className="tracker-stat-label">Critical risk trackers</span>
        </div>
        <div className="tracker-stat-divider" />
        <div className="tracker-stat">
          <span className="tracker-stat-num" style={{ color: '#f6ad55' }}>
            {TRACKER_STATS.high}
          </span>
          <span className="tracker-stat-label">High risk trackers</span>
        </div>
        <div className="tracker-stat-divider" />
        <div className="tracker-stat">
          <span className="tracker-stat-num" style={{ color: '#68d391' }}>
            {TRACKER_STATS.safeToBlock}
          </span>
          <span className="tracker-stat-label">Safe to block right now</span>
        </div>
      </div>

      {/* Domain lookup */}
      <div className="card">
        <h2>Check Any Domain</h2>
        <p className="aiwatch-normal-note">
          Paste a website URL or domain name to see if HSIP recognises it as a tracker.
        </p>
        <div className="lookup-row">
          <input
            placeholder="e.g. hotjar.com or https://www.example.com"
            value={lookup}
            onChange={e => { setLookup(e.target.value); setLookupResult(null); }}
            onKeyDown={e => e.key === 'Enter' && doLookup()}
            style={{ marginBottom: 0, flex: 1 }}
          />
          <button className="primary" onClick={doLookup} style={{ flexShrink: 0 }}>
            Check
          </button>
        </div>
        {lookupResult && <LookupResult result={lookupResult} />}
      </div>

      {/* Browser */}
      <div className="card">
        <h2>All Known Trackers</h2>

        {/* Search + filters */}
        <div className="tracker-filters">
          <input
            placeholder="Search by name or domain…"
            value={search}
            onChange={e => setSearch(e.target.value)}
            style={{ marginBottom: 0, flex: 1 }}
          />
        </div>

        <div className="filter-row">
          <div className="filter-group">
            <span className="filter-label">Category:</span>
            {CATEGORIES.map(c => (
              <button
                key={c}
                className={`filter-btn${category === c ? ' active' : ''}`}
                onClick={() => setCategory(c)}
              >
                {c}
              </button>
            ))}
          </div>
          <div className="filter-group" style={{ marginTop: '0.5rem' }}>
            <span className="filter-label">Risk:</span>
            {['All', 'Critical', 'High', 'Medium', 'Low'].map(r => (
              <button
                key={r}
                className={`filter-btn${riskFilter === r ? ' active' : ''}`}
                onClick={() => setRiskFilter(r)}
              >
                {r}
              </button>
            ))}
          </div>
        </div>

        {filtered.length === 0 ? (
          <p className="empty">No trackers match your search.</p>
        ) : (
          <div className="tracker-list">
            <div className="tracker-count">
              Showing {filtered.length} of {TRACKER_STATS.total} trackers — click any row for details
            </div>
            {filtered.map((t, i) => (
              <TrackerCard key={i} tracker={t} />
            ))}
          </div>
        )}
      </div>

      <div className="consumer-explainer card">
        <h3>What HSIP does with this information</h3>
        <p className="explainer-body">
          This database is built into HSIP's <strong>telemetry guard</strong> — a component
          that can intercept traffic to these domains and require your explicit consent before
          allowing it through. Unlike a generic firewall that blocks by IP address, HSIP
          blocks by <em>intent</em>: it knows this request is a tracking call and holds it
          until you decide. Every decision is logged in your tamper-proof audit trail.
        </p>
      </div>
    </div>
  );
}
