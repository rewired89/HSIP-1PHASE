import React, { useState } from 'react';
import { TRACKERS, RISK_LEVEL, TRACKER_STATS } from '../data/trackers';

// Wall of Shame — three most invasive trackers, chosen by category + risk
const WALL_OF_SHAME = [
  TRACKERS.find(t => t.vendor === 'Hotjar'),
  TRACKERS.find(t => t.vendor === 'FullStory'),
  TRACKERS.find(t => t.vendor === 'Facebook / Meta Pixel'),
].filter(Boolean);

// Score map: risk level → 0-100
const RISK_SCORE = { critical: 94, high: 73, medium: 47, low: 18 };
// Session recording is the most visceral — bump score
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
  const [input,  setInput]  = useState('');
  const [result, setResult] = useState(null); // null | 'idle' | { tracker } | 'clean'
  const [domain, setDomain] = useState('');

  function check() {
    if (!input.trim()) return;
    const match = lookupDomain(input);
    setDomain(input.trim());
    setResult(match ? { tracker: match } : 'clean');
    setInput('');
  }

  const score = result && result !== 'idle'
    ? (result === 'clean' ? 5 : scoreTracker(result.tracker))
    : null;
  const tier  = score !== null ? creepTier(score) : null;

  return (
    <div className="card creep-card">
      <div className="creep-header">
        <h2>Creep-O-Meter™</h2>
        <span className="creep-badge">powered by HSIP</span>
      </div>
      <p className="aiwatch-normal-note">
        Type any website and find out if it's secretly following you.
      </p>

      <div className="lookup-row" style={{ marginTop: '0.75rem' }}>
        <input
          placeholder="e.g. hotjar.com, facebook.com, your-bank.com…"
          value={input}
          onChange={e => { setInput(e.target.value); }}
          onKeyDown={e => e.key === 'Enter' && check()}
          style={{ marginBottom: 0, flex: 1 }}
        />
        <button className="primary" onClick={check} style={{ flexShrink: 0 }}>
          Check
        </button>
      </div>

      {result !== null && score !== null && (
        <div className={`creep-result creep-result--${result === 'clean' ? 'low' : result.tracker.risk}`}>
          {/* Score display */}
          <div className="creep-score-row">
            <span className="creep-score-num" style={{ color: tier.color }}>{score}</span>
            <span className="creep-score-denom">/100</span>
            <span className="creep-tier-label" style={{ color: tier.color, background: tier.bar }}>
              {tier.label}
            </span>
          </div>

          {/* Bar */}
          <div className="creep-bar-track">
            <div
              className="creep-bar-fill"
              style={{ width: `${score}%`, background: tier.color }}
            />
          </div>

          {/* Verdict */}
          {result === 'clean' ? (
            <p className="creep-verdict">
              <strong>{domain}</strong> is not in HSIP's tracker database.
              That doesn't mean it's 100% safe — just that we have no record of it being used to track you.
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
        {WALL_OF_SHAME.map((t, i) => {
          const isOpen = open === i;
          const r      = RISK_LEVEL[t.risk];
          return (
            <div
              key={i}
              className={`shame-card shame-card--${t.risk}${isOpen ? ' shame-card--open' : ''}`}
              onClick={() => setOpen(isOpen ? null : i)}
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
        })}
      </div>
    </div>
  );
}

// ── Quick actions ─────────────────────────────────────────────────────────────

const ACTIONS = [
  {
    tab:   'prove-it',
    icon:  '✍️',
    title: 'Create a Digital Alibi',
    desc:  'Prove you sent a message. Prove it wasn\'t tampered with. Useful in disputes, contracts, and court.',
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

// ── Page ──────────────────────────────────────────────────────────────────────

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

export default function HomeDashboard({ onNavigate }) {
  return (
    <div>
      <div className="consumer-hero">
        <div className="consumer-hero-icon">🛡️</div>
        <h2>Your Digital Bodyguard</h2>
        <p>
          Right now, dozens of companies are watching everything you do online.
          HSIP shows you who they are, helps you stop them, and gives you
          cryptographic proof of everything you say and do.
        </p>
      </div>

      {/* What is HSIP */}
      <div className="card" style={{ marginBottom: '1rem' }}>
        <h2>What is HSIP?</h2>
        <div className="protection-grid">
          {WHAT_IS.map((item, i) => (
            <div key={i} className="protection-item">
              <span>{item.icon}</span>
              <div>
                <strong>{item.title}</strong>
                <p>{item.body}</p>
              </div>
            </div>
          ))}
        </div>
      </div>

      <CreepMeter />
      <WallOfShame />

      <div className="card">
        <h2>What do you want to do?</h2>
        <div className="home-actions">
          {ACTIONS.map(a => (
            <button key={a.tab} className="home-action-btn" onClick={() => onNavigate(a.tab)}>
              <span className="home-action-icon">{a.icon}</span>
              <div>
                <strong>{a.title}</strong>
                <p>{a.desc}</p>
              </div>
            </button>
          ))}
        </div>
      </div>
    </div>
  );
}
