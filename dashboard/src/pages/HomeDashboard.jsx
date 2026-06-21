import React, { useState, useRef, useCallback, useEffect, useMemo } from 'react';
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
    el.style.transform = `perspective(900px) rotateY(${x}deg) rotateX(${-y}deg) translateZ(14px)`;
    el.style.transition = 'transform 0.08s ease';
  }, [strength]);
  const onMouseLeave = useCallback(() => {
    const el = ref.current;
    if (!el) return;
    el.style.transform = '';
    el.style.transition = 'transform 0.55s cubic-bezier(0.2,0,0,1)';
  }, []);
  return { ref, onMouseMove, onMouseLeave };
}

// ── Tracker data ──────────────────────────────────────────────────────────────
const WALL_OF_SHAME = [
  TRACKERS.find(t => t.vendor === 'Hotjar'),
  TRACKERS.find(t => t.vendor === 'FullStory'),
  TRACKERS.find(t => t.vendor === 'Facebook / Meta Pixel'),
].filter(Boolean);

const RISK_SCORE = { critical: 94, high: 73, medium: 47, low: 18 };
function scoreTracker(t) {
  return Math.min(100, (RISK_SCORE[t.risk] ?? 10) + (t.category === 'Session Recording' ? 5 : 0));
}
function creepTier(score) {
  if (score >= 85) return { label: 'Extremely Creepy', color: '#fc8181', bar: '#63171b' };
  if (score >= 65) return { label: 'Very Invasive',    color: '#f6ad55', bar: '#7b341e' };
  if (score >= 40) return { label: 'Suspicious',       color: '#f6e05e', bar: '#5f370e' };
  if (score >= 15) return { label: 'Low Risk',         color: '#68d391', bar: '#22543d' };
  return              { label: 'Clean',                color: '#68d391', bar: '#22543d' };
}
function lookupDomain(raw) {
  const host = raw.trim().toLowerCase().replace(/^https?:\/\//,'').replace(/\/.*$/,'').replace(/:\d+$/,'');
  return TRACKERS.find(t => {
    const p = t.domain.replace(/^\*\./,'');
    return host === p || host.endsWith('.' + p);
  }) || null;
}

// ── Action card configs ───────────────────────────────────────────────────────
const ACTIONS = [
  {
    tab:   'prove-it',
    icon:  '✍️',
    title: 'Create a Digital Alibi',
    desc:  'Prove you sent a message. Prove it was never tampered with. Court-admissible cryptographic timestamps.',
    c: { bg: 'linear-gradient(135deg,rgba(224,64,251,0.3),rgba(224,64,251,0.08))', border: '#e040fb', glow: 'rgba(224,64,251,0.3)' },
  },
  {
    tab:   'messages',
    icon:  '💬',
    title: 'Send a Signed Message',
    desc:  'Sign your words with your private key. Recipients verify it came from you — impossible to fake.',
    c: { bg: 'linear-gradient(135deg,rgba(0,229,255,0.25),rgba(0,229,255,0.06))', border: '#00e5ff', glow: 'rgba(0,229,255,0.25)' },
  },
  {
    tab:   'ai-watch',
    icon:  '🤖',
    title: 'Govern Your AI Agents',
    desc:  'Every AI action gets a cryptographic identity and audit trail. Know exactly what acted on your behalf.',
    c: { bg: 'linear-gradient(135deg,rgba(124,58,237,0.3),rgba(124,58,237,0.08))', border: '#7c3aed', glow: 'rgba(124,58,237,0.3)' },
  },
  {
    tab:   'protection',
    icon:  '🔒',
    title: 'Block All Trackers',
    desc:  `Stop ${TRACKER_STATS.safeToBlock} tracking companies from following you — system-wide, every app.`,
    c: { bg: 'linear-gradient(135deg,rgba(16,185,129,0.25),rgba(16,185,129,0.06))', border: '#10b981', glow: 'rgba(16,185,129,0.25)' },
  },
  {
    tab:   'consent-wallet',
    icon:  '🛡️',
    title: 'Manage Who Has Access',
    desc:  'See every peer who can reach you. Revoke access instantly — no emails, no excuses.',
    c: { bg: 'linear-gradient(135deg,rgba(0,229,255,0.18),rgba(224,64,251,0.12))', border: '#00e5ff', glow: 'rgba(0,229,255,0.2)' },
  },
];

// ── ATOMIC-style action card ───────────────────────────────────────────────────
function ActionCard({ tab, icon, title, desc, c, onNavigate }) {
  const tilt = useTilt(8);
  return (
    <div
      className="ac-card"
      onClick={() => onNavigate(tab)}
      ref={tilt.ref}
      onMouseMove={tilt.onMouseMove}
      onMouseLeave={tilt.onMouseLeave}
      style={{ '--ac-border': c.border, '--ac-glow': c.glow }}
    >
      <div className="ac-text">
        <strong>{title}</strong>
        <p>{desc}</p>
      </div>
      <div className="ac-circle" style={{ background: c.bg, borderColor: c.border, boxShadow: `0 0 28px ${c.glow}` }}>
        <span>{icon}</span>
      </div>
      <span className="ac-arrow">↗</span>
    </div>
  );
}

// ── Creep-O-Meter ─────────────────────────────────────────────────────────────
function CreepMeter() {
  const [input, setInput]   = useState('');
  const [result, setResult] = useState(null);
  const [domain, setDomain] = useState('');
  const [display, setDisplay] = useState(0);
  const score = result !== null && result !== 'idle'
    ? (result === 'clean' ? 5 : scoreTracker(result.tracker))
    : null;
  const tier = score !== null ? creepTier(score) : null;

  useEffect(() => {
    if (score === null) { setDisplay(0); return; }
    setDisplay(0);
    let cur = 0;
    const id = setInterval(() => {
      cur = Math.min(cur + Math.ceil(score / 18), score);
      setDisplay(cur);
      if (cur >= score) clearInterval(id);
    }, 28);
    return () => clearInterval(id);
  }, [score]);

  function check() {
    if (!input.trim()) return;
    setDomain(input.trim());
    setResult(lookupDomain(input) ? { tracker: lookupDomain(input) } : 'clean');
    setInput('');
  }

  const tilt = useTilt(5);
  return (
    <div className="card creep-card" ref={tilt.ref} onMouseMove={tilt.onMouseMove} onMouseLeave={tilt.onMouseLeave}>
      <div className="creep-header">
        <h2>Creep-O-Meter™</h2>
        <span className="creep-badge">powered by HSIP</span>
      </div>
      <p className="aiwatch-normal-note">Type any website and find out if it is secretly following you.</p>
      <div className="lookup-row" style={{ marginTop: '0.75rem' }}>
        <input placeholder="e.g. hotjar.com, facebook.com…" value={input}
          onChange={e => setInput(e.target.value)} onKeyDown={e => e.key === 'Enter' && check()}
          style={{ marginBottom: 0, flex: 1 }} />
        <button className="primary" onClick={check} style={{ flexShrink: 0 }}>Check</button>
      </div>
      {result !== null && score !== null && (
        <div className={`creep-result creep-result--${result === 'clean' ? 'low' : result.tracker.risk}`}>
          <div className="creep-score-row">
            <span className="creep-score-num" style={{ color: tier.color }}>{display}</span>
            <span className="creep-score-denom">/100</span>
            <span className="creep-tier-label" style={{ color: tier.color, background: tier.bar }}>{tier.label}</span>
          </div>
          <div className="creep-bar-track">
            <div className="creep-bar-fill" style={{ width: `${display}%`, background: tier.color, boxShadow: `0 0 14px ${tier.color}` }} />
          </div>
          {result === 'clean'
            ? <p className="creep-verdict"><strong>{domain}</strong> is not in HSIP's tracker database — no record of tracking activity.</p>
            : <div className="creep-verdict-block">
                <p className="creep-verdict"><strong>{result.tracker.vendor}</strong> — {result.tracker.plain}</p>
                <p className="creep-detail">{result.tracker.description}</p>
              </div>
          }
          <button className="consumer-reset-btn" style={{ marginTop: '0.5rem' }} onClick={() => { setResult(null); setDomain(''); }}>
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
    <div className={`shame-card shame-card--${t.risk}${isOpen ? ' shame-card--open' : ''}`}
      onClick={() => onToggle(i)} ref={tilt.ref} onMouseMove={tilt.onMouseMove} onMouseLeave={tilt.onMouseLeave}>
      <div className="shame-card-top">
        <div className="shame-rank" style={{ background: r.bg, color: r.color }}>#{i + 1}</div>
        <div className="shame-body">
          <div className="shame-vendor">{t.vendor}</div>
          <div className="shame-plain">{t.plain}</div>
        </div>
        <span className="shame-toggle">{isOpen ? '▲' : '▼'}</span>
      </div>
      {isOpen && (
        <div className="shame-detail">
          <p>{t.description}</p>
          <span className="block-badge block-badge--yes" style={{ marginTop: '0.5rem', display: 'inline-block' }}>✓ HSIP can block this</span>
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
      <p className="aiwatch-normal-note">These three are embedded on millions of sites you visit every day. Click to expose them.</p>
      <div className="shame-list">
        {WALL_OF_SHAME.map((t, i) => (
          <ShameCard key={i} t={t} i={i} isOpen={open === i} onToggle={idx => setOpen(open === idx ? null : idx)} />
        ))}
      </div>
    </div>
  );
}

// ── What is HSIP tiles ────────────────────────────────────────────────────────
const WHAT_IS = [
  { icon: '🚫', title: 'Block trackers everywhere', body: 'One switch stops Google Analytics, Facebook Pixel, TikTok, and 200+ trackers — system-wide, every app.' },
  { icon: '✍️', title: 'Tamper-proof messages',     body: 'Every message is signed with your private key. The signature proves what was said and when — valid in court.' },
  { icon: '🎙️', title: 'AI assistant integration', body: 'Connect Siri, Claude, or any AI to HSIP. Voice commands become signed, timestamped, audited messages.' },
  { icon: '🔑', title: 'Your key, your data',       body: 'HSIP runs entirely on your computer. Nothing leaves your machine. Your key is generated locally and stays there.' },
];

function WhatIsTile({ item }) {
  const tilt = useTilt(7);
  return (
    <div className="protection-item what-is-tile" ref={tilt.ref} onMouseMove={tilt.onMouseMove} onMouseLeave={tilt.onMouseLeave}>
      <span>{item.icon}</span>
      <div>
        <strong>{item.title}</strong>
        <p>{item.body}</p>
      </div>
    </div>
  );
}

// ── MISE-style portrait fan carousel ─────────────────────────────────────────
function AppShowcase({ onNavigate }) {
  const [active, setActive] = useState(2);
  const [tilt, setTilt]     = useState(0);
  const stageRef = useRef(null);

  const handleMouseMove = useCallback((e) => {
    const el = stageRef.current;
    if (!el) return;
    const r = el.getBoundingClientRect();
    setTilt(((e.clientX - r.left) / r.width - 0.5) * 7);
  }, []);

  const handleMouseLeave = useCallback(() => {
    setTilt(0);
  }, []);

  const getStyle = useMemo(() => (i) => {
    const off    = i - active;
    const absOff = Math.abs(off);
    const x      = off * 148;
    const ry     = -(off * 16) + tilt;
    const z      = off === 0 ? 42 : -absOff * 42;
    const scale  = off === 0 ? 1.06 : Math.max(0.71, 1 - absOff * 0.11);
    const op     = off === 0 ? 1    : Math.max(0.32, 1 - absOff * 0.3);
    return {
      transform: `translateX(${x}px) rotateY(${ry}deg) translateZ(${z}px) scale(${scale})`,
      opacity:   op,
      zIndex:    10 - absOff,
    };
  }, [active, tilt]);

  return (
    <div className="showcase-wrap">
      <div className="showcase-eyebrow">What do you want to do?</div>
      <div
        className="showcase-stage"
        ref={stageRef}
        onMouseMove={handleMouseMove}
        onMouseLeave={handleMouseLeave}
      >
        <div className="showcase-bg-text" aria-hidden="true">YOUR DIGITAL<br/>BODYGUARD</div>
        {ACTIONS.map((a, i) => {
          const isActive = active === i;
          return (
            <div
              key={a.tab}
              className={`sc-card${isActive ? ' sc-card--active' : ''}`}
              style={{ ...getStyle(i), '--sc-color': a.c.border, '--sc-glow': a.c.glow, '--sc-bg': a.c.bg }}
              onClick={() => isActive ? onNavigate(a.tab) : setActive(i)}
            >
              <div className="sc-card-fill" />
              {!isActive && <div className="sc-card-number">0{i + 1}</div>}
              <div className="sc-card-icon">{a.icon}</div>
              <div className="sc-card-body">
                <div className="sc-card-title">{a.title}</div>
                {isActive && <div className="sc-card-desc">{a.desc}</div>}
                {isActive && <div className="sc-card-cta">Open →</div>}
              </div>
            </div>
          );
        })}
      </div>
      <div className="showcase-dots">
        {ACTIONS.map((_, i) => (
          <button
            key={i}
            className={`sc-dot${active === i ? ' sc-dot--active' : ''}`}
            onClick={() => setActive(i)}
            aria-label={ACTIONS[i].title}
          />
        ))}
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
        <p>Right now, dozens of companies are watching everything you do online. HSIP shows you who they are, helps you stop them, and gives you cryptographic proof of everything you say and do.</p>
        <div className="hero-stat-row">
          <div className="hero-stat">
            <span className="hero-stat-num">{TRACKERS.length}</span>
            <span className="hero-stat-label">trackers mapped</span>
          </div>
          <div className="hero-stat-div" />
          <div className="hero-stat">
            <span className="hero-stat-num" style={{ color: 'var(--c-red)', textShadow: '0 0 20px rgba(244,63,94,0.5)' }}>{criticalCount}</span>
            <span className="hero-stat-label">critical risk</span>
          </div>
          <div className="hero-stat-div" />
          <div className="hero-stat">
            <span className="hero-stat-num" style={{ color: 'var(--c-cyan)', textShadow: '0 0 20px rgba(0,229,255,0.4)' }}>{TRACKER_STATS.safeToBlock}</span>
            <span className="hero-stat-label">blockable now</span>
          </div>
          <div className="hero-stat-div" />
          <div className="hero-stat">
            <span className="hero-stat-num" style={{ color: 'var(--c-green)', textShadow: '0 0 20px rgba(16,185,129,0.4)' }}>0</span>
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
            <p>HSIP provides cryptographic AI agent identity, tamper-proof audit trails, and open banking consent management.</p>
          </div>
        </div>
        <button className="fin-home-cta-btn">See Finance Overview →</button>
      </div>

      {/* ── What is HSIP ── */}
      <div className="card">
        <h2>What is HSIP?</h2>
        <div className="protection-grid">
          {WHAT_IS.map((item, i) => <WhatIsTile key={i} item={item} />)}
        </div>
      </div>

      <CreepMeter />
      <WallOfShame />

      {/* ── MISE-style app showcase ── */}
      <AppShowcase onNavigate={onNavigate} />
    </div>
  );
}
