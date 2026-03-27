import React, { useState } from 'react';

const STEPS = [
  {
    id: 'welcome',
    icon: '🔐',
    title: 'Welcome to HSIP',
    subtitle: 'Before you start, let\'s be completely transparent about what this software does on your computer.',
    content: <WelcomeStep />,
  },
  {
    id: 'stores',
    icon: '💾',
    title: 'What HSIP stores',
    subtitle: 'Here is every piece of data HSIP keeps — nothing hidden.',
    content: <StoresStep />,
  },
  {
    id: 'cannot',
    icon: '🚫',
    title: 'What HSIP cannot do',
    subtitle: 'These are hard limits built into HSIP\'s design.',
    content: <CannotStep />,
  },
  {
    id: 'local',
    icon: '🏠',
    title: 'Your data stays on YOUR machine',
    subtitle: 'HSIP is not a cloud service. There is no HSIP company server.',
    content: <LocalStep />,
  },
  {
    id: 'consent',
    icon: '✅',
    title: 'Your rights and consent',
    subtitle: 'You are in control at all times.',
    content: null, // rendered separately with checkbox
  },
];

function WelcomeStep() {
  return (
    <div className="ob-content">
      <p className="ob-intro">
        HSIP runs a small server on <strong>your own computer</strong> (localhost).
        It doesn't connect to any external HSIP company server.
        Think of it like an app you install — everything stays local.
      </p>
      <div className="ob-what-grid">
        <div className="ob-what-item ob-what-yes">
          <span>✍️</span>
          <div>
            <strong>Signs your messages</strong>
            <p>Creates cryptographic proof that a message came from you</p>
          </div>
        </div>
        <div className="ob-what-item ob-what-yes">
          <span>🛡️</span>
          <div>
            <strong>Manages your consents</strong>
            <p>Records who you give permission to, with timestamps</p>
          </div>
        </div>
        <div className="ob-what-item ob-what-yes">
          <span>🤖</span>
          <div>
            <strong>Controls AI access</strong>
            <p>Tracks which AI agents have permission to use your account</p>
          </div>
        </div>
        <div className="ob-what-item ob-what-yes">
          <span>🔍</span>
          <div>
            <strong>Identifies trackers</strong>
            <p>Recognises 24+ known tracking companies by domain</p>
          </div>
        </div>
      </div>
    </div>
  );
}

function StoresStep() {
  return (
    <div className="ob-content">
      <div className="ob-list">
        <div className="ob-list-item ob-list-yes">
          <span className="ob-list-icon">🔑</span>
          <div>
            <strong>Your API key</strong>
            <p>
              Saved in your browser's localStorage so you stay logged in.
              It never leaves your device to an external server.
            </p>
          </div>
        </div>
        <div className="ob-list-item ob-list-yes">
          <span className="ob-list-icon">🪪</span>
          <div>
            <strong>A cryptographic identity (key pair)</strong>
            <p>
              An Ed25519 key pair is generated and stored in HSIP's local database
              (SQLite file on your computer). The private key is encrypted
              at rest — HSIP cannot read it without your master key.
            </p>
          </div>
        </div>
        <div className="ob-list-item ob-list-yes">
          <span className="ob-list-icon">📋</span>
          <div>
            <strong>Consent records you create</strong>
            <p>
              When you grant or revoke access for someone, that record is stored locally
              with a timestamp and cryptographic signature.
            </p>
          </div>
        </div>
        <div className="ob-list-item ob-list-yes">
          <span className="ob-list-icon">✍️</span>
          <div>
            <strong>Messages you choose to sign</strong>
            <p>
              When you use "Prove It" to sign a message, the message text and its
              signature are stored in the local database so you can reference them later.
            </p>
          </div>
        </div>
        <div className="ob-list-item ob-list-yes">
          <span className="ob-list-icon">📜</span>
          <div>
            <strong>An audit log of all actions</strong>
            <p>
              Every operation (sign, verify, grant, revoke) is appended to a
              hash-chained audit log. This log cannot be edited — it's append-only by design.
            </p>
          </div>
        </div>
      </div>
    </div>
  );
}

function CannotStep() {
  return (
    <div className="ob-content">
      <div className="ob-list">
        {[
          ['📁', 'Read your files or folders',
            'HSIP has no file system access. It only reads and writes its own database.'],
          ['📸', 'Access your camera or microphone',
            'HSIP is a backend server — it has no browser permissions and cannot access media devices.'],
          ['⌨️', 'Log your keystrokes',
            'HSIP has no keyboard hooks or input monitoring of any kind.'],
          ['🌐', 'See your browsing history',
            'HSIP doesn\'t integrate with your browser unless you explicitly configure a proxy. Even then, it only inspects domain names — not the content of pages.'],
          ['🔐', 'Access your passwords or other apps',
            'HSIP is isolated — it communicates only with its own local database and the dashboard. It cannot read data from other apps.'],
          ['☁️', 'Send your data to an external server',
            'There is no HSIP cloud. The server you\'re talking to is localhost:3000 — your own machine. No data leaves unless you explicitly export it.'],
        ].map(([icon, title, desc]) => (
          <div key={title} className="ob-list-item ob-list-no">
            <span className="ob-list-icon">{icon}</span>
            <div>
              <strong>{title}</strong>
              <p>{desc}</p>
            </div>
          </div>
        ))}
      </div>
    </div>
  );
}

function LocalStep() {
  return (
    <div className="ob-content">
      <div className="ob-local-diagram">
        <div className="ob-local-box ob-local-you">
          <span>💻</span>
          <strong>Your Computer</strong>
          <small>localhost</small>
        </div>
        <div className="ob-local-arrow">
          <div className="ob-local-arrow-line" />
          <div className="ob-local-arrow-label">All communication stays here</div>
        </div>
        <div className="ob-local-box ob-local-hsip">
          <span>🔐</span>
          <strong>HSIP Server</strong>
          <small>localhost:3000</small>
        </div>
      </div>
      <div className="ob-list" style={{ marginTop: '1.5rem' }}>
        <div className="ob-list-item ob-list-yes">
          <span className="ob-list-icon">✅</span>
          <div>
            <strong>You own the server</strong>
            <p>You downloaded HSIP, you run it, you control it. There is no company that can turn it off, read your data, or change how it works.</p>
          </div>
        </div>
        <div className="ob-list-item ob-list-yes">
          <span className="ob-list-icon">✅</span>
          <div>
            <strong>Open source — the code is public</strong>
            <p>Every line of code is on GitHub. You (or anyone you trust) can read exactly what HSIP does. There are no hidden features.</p>
          </div>
        </div>
        <div className="ob-list-item ob-list-yes">
          <span className="ob-list-icon">✅</span>
          <div>
            <strong>Delete everything, any time</strong>
            <p>The "Erase All My Data" feature (GDPR Article 17) wipes every record HSIP has — consents, messages, identities, audit log — instantly and permanently.</p>
          </div>
        </div>
      </div>
    </div>
  );
}

function ConsentStep({ agreed, onToggle }) {
  return (
    <div className="ob-content">
      <div className="ob-consent-list">
        {[
          'HSIP stores a cryptographic key pair and consent records in a local database on my computer.',
          'HSIP\'s audit log is append-only — I cannot delete individual entries, only erase all data at once.',
          'HSIP only communicates with localhost. No data is sent to any external server.',
          'I can delete all my HSIP data at any time using the "Erase All My Data" option.',
          'HSIP is open source software provided as-is, without warranty.',
        ].map((item, i) => (
          <div key={i} className="ob-consent-item">
            <span className="ob-consent-check">✓</span>
            <p>{item}</p>
          </div>
        ))}
      </div>

      <label className="ob-agree-label">
        <input
          type="checkbox"
          checked={agreed}
          onChange={onToggle}
          className="ob-agree-checkbox"
        />
        <span>
          I have read and understood what HSIP does on my machine, and I consent to these operations.
        </span>
      </label>
    </div>
  );
}

export default function Onboarding({ onComplete }) {
  const [step,   setStep]   = useState(0);
  const [agreed, setAgreed] = useState(false);

  const current = STEPS[step];
  const isLast  = step === STEPS.length - 1;
  const isFirst = step === 0;

  function finish() {
    if (!agreed) return;
    localStorage.setItem('hsip_onboarding_done', '1');
    onComplete();
  }

  return (
    <div className="ob-overlay">
      <div className="ob-modal">
        {/* Progress dots */}
        <div className="ob-progress">
          {STEPS.map((s, i) => (
            <div
              key={s.id}
              className={`ob-dot${i === step ? ' ob-dot--active' : i < step ? ' ob-dot--done' : ''}`}
            />
          ))}
        </div>

        {/* Step header */}
        <div className="ob-header">
          <div className="ob-step-icon">{current.icon}</div>
          <h2 className="ob-title">{current.title}</h2>
          <p className="ob-subtitle">{current.subtitle}</p>
        </div>

        {/* Step content */}
        <div className="ob-body">
          {isLast
            ? <ConsentStep agreed={agreed} onToggle={() => setAgreed(v => !v)} />
            : current.content}
        </div>

        {/* Navigation */}
        <div className="ob-footer">
          {!isFirst && (
            <button className="ob-back-btn" onClick={() => setStep(s => s - 1)}>
              ← Back
            </button>
          )}
          <div style={{ flex: 1 }} />
          {isLast ? (
            <button
              className="primary ob-finish-btn"
              onClick={finish}
              disabled={!agreed}
            >
              Start using HSIP →
            </button>
          ) : (
            <button className="primary ob-next-btn" onClick={() => setStep(s => s + 1)}>
              Next →
            </button>
          )}
        </div>
      </div>
    </div>
  );
}
