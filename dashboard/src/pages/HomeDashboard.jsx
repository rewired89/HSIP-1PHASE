import React, { useState, useRef, useCallback, useEffect } from 'react';
import { TRACKERS, RISK_LEVEL, TRACKER_STATS } from '../data/trackers';

// ── 3D tilt on mouse move ─────────────────────────────────────────────────────
function useTilt(strength = 10) {
  const ref = useRef(null);

  const onMouseMove = useCallback((e) => {
    const el = ref.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    const x = ((e.clientX - r.left) / r.width  - 0.5) * strength;
    const y = ((e.clientY - r.top)  / r.height - 0.5) * strength;
    el.style.transform = `perspective(800px) rotateY(${x}deg) rotateX(${-y}deg) translateZ(12px)`;
    el.style.transition = 'transform 0.08s ease';
  }, [strength]);

  const onMouseLeave = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    el.style.transform = '';
    el.style.transition = 'transform 0.5s cubic-bezier(0.2, 0, 0, 1)';
  }, []);

  return { ref, onMouseMove, onMouseLeave };
}

// Wall of Shame — three most invasive trackers
const WALL_OF_SHAME = [
  TRACKERS.find(t => t.vendor === 'Hotjar'),
  TRACKERS.find(t => t.vendor === 'FullStory'),
  TRACKERS.find(t => t.vendor === 'Facebook / Meta Pixel'),
].filter(Boolean);

const RISK_SCORE = { critical: 94, high: 73, medium: 47, low: 18 };
function scoreTracker(t) {
  const base = RISK_SCORE[t.risk] ?? 10;
  return Math.min(100, base + (t.category === 'Session Recording' ? 5 : 0));
}

function creepTier(score) {
  if (score >= 85) return { label: 'Extremely Creepy',  color: '#fc8181', bar: '#63171b' };
  if (score >= 65) return { label: 'Very Invasive',     color: '#f6ad55', bar: '#7b341e' };
  if (score >= 40) return { label: 'Suspicious',        color: '#f6e05e', bar: '#5f370e' };
  if (score >= 15) return { label: 'Low Risk',          color: '#68d391', bar: '#22543d' };
  return              { label: 'Clean',                 color: '#68d391', bar: '#22543d' };
}

function lookupDomain(raw) {
  const host = raw.trim().toLowerCase()
    .replace(/^https?:\/\//, '')
    .replace(/\/.*$/, '')
    .replace(/:\d+$/, '');
  return TRACKERS.find(t => {
    const pattern = t.domain.replace(/^\*\./, '');
    return host === pattern || host.endsWith('.' + pattern);
  }) || null;
}

// ── Creep-O-Meter ─────────────────────────────────────────────────────────────
function CreepMeter() {
  const [input,        setInput]       = useState('');
  const [result,       setResult]      = useState(null);
  const [domain,       setDomain]      = useState('');
  const [displayScore, setDisplayScore] = useState(0);

  const score = result !== null && result !== 'idle'
    ? (result === 'clean' ? 5 : scoreTracker(result.tracker))
    : null;
  const tier = score !== null ? creepTier(score) : null;

  // Animate score counting up
  useEffect(() => {
    if (score === null) { setDisplayScore(0); return; }
    setDisplayScore(0);
    let current = 0;
    const target = score;
    const id = setInterval(() => {
      current = Math.min(current + Math.ceil(target / 18), target);
      setDisplayScore(current);
      if (current >= target) clearInterval(id);
    }, 28);
    return () => clearInterval(id);
  }, [score]);

  function check() {
    if (!input.trim()) return;
    const match = lookupDomain(input);
    setDomain(input.trim());
    setResult(match ? { tracker: match } : 'clean');
    setInput('');
  }

  const tilt = useTilt(5);

  return (
    <div className="card creep-card" ref={tilt.ref} onMouseMove={tilt.onMouseMove} onMouseLeave={tilt.onMouseLeave}>
      <div className="creep-header">
        <h2>Creep-O-Meter™</h2>
        <span className="creep-badge">powered by HSIP</span>
      </div>
      <p className="aiwatch-normal-note">
        Type any website and find out if it is secretly following you.
      </p>

      <div className="lookup-row" style={{ marginTop: '0.75rem' }}>
        <input
          placeholder="e.g. hotjar.com, facebook.com, your-bank.com…"
          value={input}
          onChange={e => setInput(e.target.value)}
          onKeyDown={e => e.key === 'Enter' && check()}
          style={{ marginBottom: 0, flex: 1 }}
        />
        <button className="primary" onClick={check} style={{ flexShrink: 0 }}>
          Check
        </button>
      </div>

      {result !== null && score !== null && (
        <div className={`creep-result creep-result--${result === 'clean' ? 'low' : result.tracker.risk}`}>
          <div className="creep-score-row">
            <span className="creep-score-num" style={{ color: tier.color }}>{displayScore}</span>
            <span className="creep-score-denom">/100</span>
            <span className="creep-tier-label" style={{ color: tier.color, background: tier.bar }}>
              {tier.label}
            </span>
          </div>

          <div className="creep-bar-track">
            <div
              className="creep-bar-fill"
              style={{ width: `${displayScore}%`, background: tier.color, boxShadow: `0 0 12px ${tier.color}` }}
            />
          </div>

          {result === 'clean' ? (
            <p className="creep-verdict">
              <strong>{domain}</strong> is not in HSIP's tracker database.
              That does not mean it is 100% safe — just that we have no record of it being used to track you.
            </p>
          ) : (
            <div className="creep-verdict-block">
              <p className="creep-verdict">
                <strong>{result.tracker.vendor}</strong> — {result.tracker.plain}
              </p>
              <p className="creep-detail">{result.tracker.description}</p>
            </div>
          )}

          <button
            className="consumer-reset-btn"
            style={{ marginTop: '0.5rem' }}
            onClick={() => { setResult(null); setDomain(''); }}
          >
            Check another site
          </button>
        </div>
      )}
    </div>
  );
}

// ── Wall of Shame ─────────────────────────────────────────────────────────────
function ShameCard({ t, i, isOpen, onToggle }) {
  const tilt = useTilt(6);
  const r = RISK_LEVEL[t.risk];
  return (
    <div
      className={`shame-card shame-card--${t.risk}${isOpen ? ' shame-card--open' : ''}`}
      onClick={() => onToggle(i)}
      ref={tilt.ref}
      onMouseMove={tilt.onMouseMove}
      onMouseLeave={tilt.onMouseLeave}
    >
      <div className="shame-card-top">
        <div className="shame-rank" style={{ background: r.bg, color: r.color }}>
          #{i + 1}
        </div>
        <div className="shame-body">
          <div className="shame-vendor">{t.vendor}</div>
          <div className="shame-plain">{t.plain}</div>
        </div>
        <span className="shame-toggle">{isOpen ? '▲' : '▼'}</span>
      </div>
      {isOpen && (
        <div className="shame-detail">
          <p>{t.description}</p>
          <span className="block-badge block-badge--yes" style={{ marginTop: '0.5rem', display: 'inline-block' }}>
            ✓ HSIP can block this
          </span>
        </div>
      )}
    </div>
  );
}

function WallOfShame() {
  const [open, setOpen] = useState(null);

  return (
    <div className="card">
      <h2>🏴 Wall of Shame</h2>
      <p className="aiwatch-normal-note">
        These three are embedded on millions of sites you visit every day.
        Click any to see exactly what they do to you.
      </p>
      <div className="shame-list">
        {WALL_OF_SHAME.map((t, i) => (
          <ShameCard
            key={i}
            t={t}
            i={i}
            isOpen={open === i}
            onToggle={idx => setOpen(open === idx ? null : idx)}
          />
        ))}
      </div>
    </div>
  );
}

// ── Quick action tile with tilt ───────────────────────────────────────────────
function ActionTile({ tab, icon, title, desc, onNavigate }) {
  const tilt = useTilt(8);
  return (
    <button
      className="home-action-btn"
      onClick={() => onNavigate(tab)}
      ref={tilt.ref}
      onMouseMove={tilt.onMouseMove}
      onMouseLeave={tilt.onMouseLeave}
    >
      <span className="home-action-icon">{icon}</span>
      <div>
        <strong>{title}</strong>
        <p>{desc}</p>
      </div>
    </button>
  );
}

const ACTIONS = [
  {
    tab:   'prove-it',
    icon:  '✍️',
    title: 'Create a Digital Alibi',
    desc:  'Prove you sent a message. Prove it was not tampered with. Useful in disputes, contracts, and court.',
  },
  {
    tab:   'messages',
    icon:  '💬',
    title: 'Send a Signed Message',
    desc:  'Send a message signed with your private key. The recipient can verify it came from you and was never changed.',
  },
  {
    tab:   'ai-watch',
    icon:  '🤖',
    title: 'Connect Your AI Assistant',
    desc:  'Let Siri, Claude, or any AI send signed messages on your behalf — even by voice command.',
  },
  {
    tab:   'protection',
    icon:  '🔒',
    title: 'Block All Trackers',
    desc:  `Stop ${TRACKER_STATS.safeToBlock} tracking companies from following you. Takes 5 minutes, works on every app.`,
  },
  {
    tab:   'consent-wallet',
    icon:  '🛡️',
    title: 'Manage Who Has Access',
    desc:  'See who can reach you and remove their access instantly — no emails, no waiting, no excuses.',
  },
];

const WHAT_IS = [
  {
    icon: '🚫',
    title: 'Block trackers everywhere',
    body:  'One switch stops Google Analytics, Facebook Pixel, TikTok, and 200+ trackers — system-wide, every app, not just your browser.',
  },
  {
    icon: '✍️',
    title: 'Tamper-proof messages',
    body:  'Every message you send is signed with your private key. The signature proves exactly what was said and when — useful in court, contracts, or any dispute.',
  },
  {
    icon: '🎙️',
    title: 'AI assistant integration',
    body:  'Connect Siri, Claude, or any AI to HSIP. Say "Hey Siri, send HSIP message" and your words are signed and timestamped instantly.',
  },
  {
    icon: '🔑',
    title: 'Your key, your data',
    body:  'HSIP runs entirely on your computer. Nothing leaves your machine. Your cryptographic identity key is generated locally and stays there.',
  },
];

// ── What-is tile with tilt ────────────────────────────────────────────────────
function WhatIsTile({ item }) {
  const tilt = useTilt(7);
  return (
    <div
      className="protection-item what-is-tile"
      ref={tilt.ref}
      onMouseMove={tilt.onMouseMove}
      onMouseLeave={tilt.onMouseLeave}
    >
      <span>{item.icon}</span>
      <div>
        <strong>{item.title}</strong>
        <p>{item.body}</p>
      </div>
    </div>
  );
}

// ── Page ──────────────────────────────────────────────────────────────────────
export default function HomeDashboard({ onNavigate }) {
  const criticalCount = TRACKERS.filter(t => t.risk === 'critical').length;

  return (
    <div>
      {/* ── Hero ── */}
      <div className="consumer-hero">
        <div className="consumer-hero-icon">🛡️</div>
        <h2>Your Digital Bodyguard</h2>
        <p>
          Right now, dozens of companies are watching everything you do online.
          HSIP shows you who they are, helps you stop them, and gives you
          cryptographic proof of everything you say and do.
        </p>
        <div className="hero-stat-row">
          <div className="hero-stat">
            <span className="hero-stat-num">{TRACKERS.length}</span>
            <span className="hero-stat-label">trackers mapped</span>
          </div>
          <div className="hero-stat-div" />
          <div className="hero-stat">
            <span className="hero-stat-num" style={{ color: 'var(--c-red)' }}>{criticalCount}</span>
            <span className="hero-stat-label">critical risk</span>
          </div>
          <div className="hero-stat-div" />
          <div className="hero-stat">
            <span className="hero-stat-num">{TRACKER_STATS.safeToBlock}</span>
            <span className="hero-stat-label">blockable now</span>
          </div>
          <div className="hero-stat-div" />
          <div className="hero-stat">
            <span className="hero-stat-num" style={{ color: 'var(--c-green)' }}>0</span>
            <span className="hero-stat-label">data sold by us</span>
          </div>
        </div>
      </div>

      {/* ── Finance CTA ── */}
      <div className="card fin-home-cta" onClick={() => onNavigate('finance')}>
        <div className="fin-home-cta-left">
          <span className="fin-home-cta-icon">🏦</span>
          <div className="fin-home-cta-inner">
            <span className="fin-home-cta-badge">Financial Services</span>
            <strong>Building for banks, fintechs, or trading desks?</strong>
            <p>
              HSIP provides cryptographic AI agent identity, tamper-proof audit trails,
              and open banking consent management — SOX, FINRA, MiFID II, and PSD2 ready.
            </p>
          </div>
        </div>
        <button className="fin-home-cta-btn">See Finance Overview →</button>
      </div>

      {/* ── What is HSIP ── */}
      <div className="card" style={{ marginBottom: '1rem' }}>
        <h2>What is HSIP?</h2>
        <div className="protection-grid">
          {WHAT_IS.map((item, i) => (
            <WhatIsTile key={i} item={item} />
          ))}
        </div>
      </div>

      <CreepMeter />
      <WallOfShame />

      {/* ── Quick actions ── */}
      <div className="card">
        <h2>What do you want to do?</h2>
        <div className="home-actions">
          {ACTIONS.map(a => (
            <ActionTile key={a.tab} {...a} onNavigate={onNavigate} />
          ))}
        </div>
      </div>
    </div>
  );
}
