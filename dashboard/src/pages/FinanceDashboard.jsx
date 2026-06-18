import React, { useState } from 'react';
import { request } from '../api';

// ── Market gap data ───────────────────────────────────────────────────────────

const MARKET_GAPS = [
  {
    icon: '🤖',
    title: 'AI agents have no cryptographic identity',
    tags: ['MiFID II Art. 25', 'SOX §404'],
    problem: 'Banks deploy hundreds of AI agents — trading bots, risk models, LLM chatbots — but none carry a verifiable cryptographic identity. When a model makes a bad call, there is no signed record linking the decision to a specific agent version or key.',
    solution: 'HSIP issues each agent an Ed25519 keypair on registration. Every action is signed and timestamped. Anomaly velocity checks auto-revoke compromised agents in real time — no manual intervention.',
    effort: '5 min setup',
  },
  {
    icon: '🏦',
    title: 'Open Banking consent is an OAuth nightmare',
    tags: ['PSD2', 'Open Finance', 'GDPR Art. 7'],
    problem: 'PSD2 mandates granular, revocable consent for data sharing. Existing flows are multi-step OAuth redirects. Users cannot see what they have consented to, cannot revoke it instantly, and there is no cryptographic proof of the consent event itself.',
    solution: 'HSIP provides time-bounded cryptographic consent: issue, revoke, and verify consent in a single API call. Full consent lifecycle is stored in an immutable signed audit log. Revocation takes effect immediately — not after a token TTL.',
    effort: '1 API call',
  },
  {
    icon: '📋',
    title: 'Algorithmic trading logs are not tamper-proof',
    tags: ['MiFID II Art. 25', 'FINRA Rule 4511'],
    problem: 'MiFID II requires documenting algorithmic trading decisions. Most banks write decision logs to a SQL database. A privileged DBA or a compromised database can silently alter those records — and the alteration leaves no trace.',
    solution: 'HSIP signs every log entry with Ed25519 before storing it. The signature chain makes retroactive modification detectable. No blockchain overhead, no cloud dependency — just applied cryptography.',
    effort: '1 line of code',
  },
  {
    icon: '🔗',
    title: 'No lightweight protocol for inter-bank agent trust',
    tags: ['SWIFT gpi', 'FedNow', 'ISO 20022'],
    problem: 'When your AI sends a settlement instruction to a counterparty system, how does the counterparty verify it actually came from your authorised agent and not a spoofed request? No lightweight, open, self-hostable protocol exists for this today.',
    solution: 'HSIP federated trust lets institutions exchange Ed25519 verify keys out-of-band and verify signatures locally against a label. No central authority. No PKI infrastructure. Two API calls to establish and one to verify.',
    effort: '2 API calls',
  },
  {
    icon: '⚡',
    title: 'Rogue AI detection is reactive, not preventive',
    tags: ['DORA', 'Operational Resilience', 'SR 11-7'],
    problem: 'When an AI agent starts making thousands of requests per minute — due to a bug, a prompt injection attack, or an adversarial loop — banks have no system-level circuit breaker. Existing SIEM solutions detect incidents hours after damage is done.',
    solution: 'HSIP tracks per-agent request velocity in a sliding window. At >100 req/min it logs an anomaly. At >1000 req/min it auto-revokes the key and writes a signed incident record — blocking the agent before the incident escalates.',
    effort: 'Built-in, zero config',
  },
];

const USE_CASES = [
  {
    icon: '🤖',
    title: 'AI Trading Bot Identity',
    tagline: 'Every bot gets a passport',
    body: 'Register each trading model version as an HSIP agent. Its Ed25519 key signs every order it generates. MiFID II algorithm documentation becomes a verifiable, signed audit trail — not a narrative in a PDF.',
    api: 'POST /v1/keys  { agent_type: "ai_agent" }',
  },
  {
    icon: '🏦',
    title: 'Open Banking Consent',
    tagline: 'PSD2-compliant in one call',
    body: 'Issue time-bounded cryptographic consent when a customer authorises a TPP. Revoke instantly with a single API call. The full consent lifecycle — grant, check, revoke — lives in the append-only audit log.',
    api: 'POST /v1/consent/grant  →  POST /v1/consent/revoke',
  },
  {
    icon: '✍️',
    title: 'Wire Transfer & Trade Signing',
    tagline: 'Non-repudiation for every instruction',
    body: 'Sign wire transfer instructions and trade orders with Ed25519 before they enter the payments rail. The signature proves the instruction was authorised by the keyholder and was not tampered with in transit — satisfying SWIFT CSCF controls.',
    api: 'POST /v1/messages/sign',
  },
  {
    icon: '📋',
    title: 'Regulatory Audit Trail',
    tagline: 'SOX · FINRA · MiFID II out of the box',
    body: 'Every state-changing event — key creation, consent grant, credential issuance, anomaly flag — writes a signed, append-only entry to the audit log automatically. No instrumentation needed beyond using the API.',
    api: 'GET /v1/audit?limit=500',
  },
  {
    icon: '🔗',
    title: 'Cross-bank Federated Trust',
    tagline: 'Verify counterparty agents without a central authority',
    body: 'Exchange Ed25519 verify keys with correspondent banks or clearing houses out-of-band. Verify any message from their AI agents locally — no round trips to a central registry, no vendor lock-in, no SLA dependency.',
    api: 'POST /v1/trust/peer  →  POST /v1/trust/verify',
  },
  {
    icon: '🪪',
    title: 'KYC Verifiable Credentials',
    tagline: 'Portable identity without centralisation',
    body: 'Issue verifiable credentials for KYC-cleared customers. The credential carries an Ed25519 signature from the issuing institution. Any counterparty can verify it locally against the issuer public key — no API round trip required.',
    api: 'POST /v1/credentials/issue  →  POST /v1/credentials/verify',
  },
];

const REGULATIONS = [
  { code: 'SOX §404',          label: 'Audit trail integrity',   status: 'covered' },
  { code: 'FINRA Rule 4511',   label: 'Books & records',         status: 'covered' },
  { code: 'MiFID II Art. 25',  label: 'Algo documentation',      status: 'covered' },
  { code: 'PSD2 / Open Finance', label: 'Consent management',    status: 'covered' },
  { code: 'GDPR Art. 7',       label: 'Consent withdrawal',      status: 'covered' },
  { code: 'DORA',              label: 'AI system resilience',    status: 'partial'  },
  { code: 'SWIFT CSCF',        label: 'Access controls',         status: 'partial'  },
  { code: 'ISO 20022',         label: 'Structured payments',     status: 'roadmap'  },
];

const COMPARISON_ROWS = [
  { feature: 'Ed25519 cryptographic identity per AI agent', hsip: true,  central: true,  chain: true,  nothing: false },
  { feature: 'Zero cloud dependency — fully on-prem',       hsip: true,  central: false, chain: false, nothing: true  },
  { feature: 'Tamper-proof signed audit log',               hsip: true,  central: false, chain: true,  nothing: false },
  { feature: 'AI agent velocity anomaly detection',         hsip: true,  central: false, chain: false, nothing: false },
  { feature: 'Instant consent revocation',                  hsip: true,  central: true,  chain: false, nothing: false },
  { feature: 'Federated inter-institution trust',           hsip: true,  central: false, chain: true,  nothing: false },
  { feature: 'Zero-config single binary',                   hsip: true,  central: false, chain: false, nothing: true  },
  { feature: 'Open source / auditable',                     hsip: true,  central: false, chain: false, nothing: true  },
];

// ── Sub-components ────────────────────────────────────────────────────────────

function GapCard({ gap, index }) {
  const [open, setOpen] = useState(false);

  return (
    <div
      className={`fin-gap-card${open ? ' fin-gap-card--open' : ''}`}
      onClick={() => setOpen(o => !o)}
    >
      <div className="fin-gap-header">
        <span className="fin-gap-num">{index + 1}</span>
        <span className="fin-gap-icon">{gap.icon}</span>
        <div className="fin-gap-title-block">
          <span className="fin-gap-title">{gap.title}</span>
          <div className="fin-gap-tags">
            {gap.tags.map(t => (
              <span key={t} className="fin-reg-chip">{t}</span>
            ))}
          </div>
        </div>
        <span className="fin-gap-toggle">{open ? '▲' : '▼'}</span>
      </div>
      {open && (
        <div className="fin-gap-body">
          <div className="fin-gap-problem">
            <span className="fin-gap-label">THE GAP</span>
            <p>{gap.problem}</p>
          </div>
          <div className="fin-gap-solution">
            <span className="fin-gap-label fin-gap-label--solution">HSIP FILLS IT</span>
            <p>{gap.solution}</p>
            <span className="fin-effort-badge">{gap.effort}</span>
          </div>
        </div>
      )}
    </div>
  );
}

function UseCaseCard({ uc, active, onToggle }) {
  return (
    <div
      className={`fin-uc-card${active ? ' fin-uc-card--active' : ''}`}
      onClick={onToggle}
    >
      <div className="fin-uc-icon">{uc.icon}</div>
      <div className="fin-uc-content">
        <div className="fin-uc-title">{uc.title}</div>
        <div className="fin-uc-tagline">{uc.tagline}</div>
        {active && (
          <>
            <p className="fin-uc-body">{uc.body}</p>
            <code className="fin-uc-api">{uc.api}</code>
          </>
        )}
      </div>
    </div>
  );
}

function RegBadge({ reg }) {
  const colours = {
    covered: { bg: 'rgba(34,197,94,0.1)',    color: '#22c55e', dot: '#22c55e' },
    partial:  { bg: 'rgba(234,179,8,0.1)',   color: '#eab308', dot: '#eab308' },
    roadmap:  { bg: 'rgba(99,102,241,0.1)',  color: '#818cf8', dot: '#818cf8' },
  };
  const c = colours[reg.status];
  return (
    <div className="fin-reg-badge" style={{ background: c.bg, borderColor: c.dot + '44' }}>
      <span className="fin-reg-dot" style={{ background: c.dot }} />
      <div>
        <div className="fin-reg-code" style={{ color: c.color }}>{reg.code}</div>
        <div className="fin-reg-label">{reg.label}</div>
      </div>
    </div>
  );
}

function CompCell({ val }) {
  if (val === true)  return <span className="fin-cmp-yes">✓</span>;
  if (val === false) return <span className="fin-cmp-no">✗</span>;
  return <span className="fin-cmp-partial">~</span>;
}

function ComparisonTable() {
  return (
    <div className="fin-cmp-wrap">
      <table className="fin-cmp-table">
        <thead>
          <tr>
            <th>Feature</th>
            <th className="fin-cmp-hsip">HSIP</th>
            <th>Central Auth + HSM</th>
            <th>Blockchain</th>
            <th>Nothing (today)</th>
          </tr>
        </thead>
        <tbody>
          {COMPARISON_ROWS.map((r, i) => (
            <tr key={i}>
              <td>{r.feature}</td>
              <td className="fin-cmp-hsip"><CompCell val={r.hsip} /></td>
              <td><CompCell val={r.central} /></td>
              <td><CompCell val={r.chain} /></td>
              <td><CompCell val={r.nothing} /></td>
            </tr>
          ))}
        </tbody>
      </table>
    </div>
  );
}

// ── Live Demo ─────────────────────────────────────────────────────────────────

const DEMO_SCENARIOS = [
  {
    label: 'Trade Order',
    text: 'BUY 1,000 AAPL @ $185.50 — Risk Model v2.1 — Desk: Equities NYC — Ref: TRD-2024-088421',
  },
  {
    label: 'Wire Transfer',
    text: 'WIRE USD 2,500,000 to JP Morgan NYC — Swift: CHASUS33 — Value Date: 2024-01-16 — Ref: IB-TRF-20240115-007',
  },
  {
    label: 'Open Banking Consent',
    text: 'CONSENT GRANTED: Customer C-98234 authorises Plaid Inc read-only access to account balances for 90 days. Expires 2024-04-15T00:00:00Z.',
  },
  {
    label: 'Credit Decision',
    text: 'CREDIT APPROVAL: Customer 4421-B approved for $50,000 revolving credit at 14.9% APR — Model: UnderwriteBot v3 — Confidence: 0.94',
  },
];

function LiveSignDemo({ apiKey }) {
  const [scenario,    setScenario]    = useState(0);
  const [instruction, setInstruction] = useState(DEMO_SCENARIOS[0].text);
  const [result,      setResult]      = useState(null);
  const [busy,        setBusy]        = useState(false);
  const [verified,    setVerified]    = useState(null);

  function pickScenario(i) {
    setScenario(i);
    setInstruction(DEMO_SCENARIOS[i].text);
    setResult(null);
    setVerified(null);
  }

  async function sign() {
    setBusy(true);
    setResult(null);
    setVerified(null);
    try {
      const [signed, id] = await Promise.all([
        request('POST', '/v1/messages/sign', { content: instruction }, apiKey),
        request('GET',  '/v1/identity',     null,                    apiKey),
      ]);
      setResult({ ...signed, verify_key: id.verify_key, signed_at: new Date().toISOString() });
    } catch (e) {
      alert(e.message || 'Sign failed — is the HSIP server running?');
    }
    setBusy(false);
  }

  async function verify() {
    if (!result) return;
    setBusy(true);
    try {
      const r = await request('POST', '/v1/messages/verify', {
        content:         instruction,
        signature:       result.signature,
        peer_verify_key: result.verify_key,
      }, apiKey);
      setVerified(r.verified);
    } catch {
      setVerified(false);
    }
    setBusy(false);
  }

  return (
    <div className="card fin-demo-card">
      <h2>Live Demo — Sign a Financial Instruction</h2>
      <p className="fin-demo-desc">
        Pick a scenario or type your own. HSIP signs it with Ed25519 — the same algorithm
        used in TLS 1.3 and modern HSMs. The resulting signature proves the instruction was
        issued by the keyholder and was not altered in transit.
      </p>

      <div className="fin-demo-scenarios">
        {DEMO_SCENARIOS.map((s, i) => (
          <button
            key={i}
            className={`fin-scenario-btn${scenario === i ? ' fin-scenario-btn--active' : ''}`}
            onClick={() => pickScenario(i)}
          >
            {s.label}
          </button>
        ))}
      </div>

      <textarea
        rows={3}
        value={instruction}
        onChange={e => {
          setInstruction(e.target.value);
          setResult(null);
          setVerified(null);
        }}
        style={{ marginTop: '0.75rem', fontFamily: 'monospace', fontSize: '0.8rem' }}
      />

      <div className="fin-demo-actions">
        <button
          className="primary"
          onClick={sign}
          disabled={busy || !instruction.trim()}
        >
          {busy && !result ? 'Signing…' : 'Sign Instruction'}
        </button>
        {result && (
          <button
            className="primary fin-verify-btn"
            onClick={verify}
            disabled={busy}
          >
            {busy && result ? 'Verifying…' : 'Verify Signature'}
          </button>
        )}
      </div>

      {result && (
        <div className="fin-result-block">
          <div className="fin-result-row">
            <span className="fin-result-label">Algorithm</span>
            <span className="fin-result-val">Ed25519 (RFC 8032)</span>
          </div>
          <div className="fin-result-row">
            <span className="fin-result-label">Signed at</span>
            <span className="fin-result-val">{result.signed_at}</span>
          </div>
          <div className="fin-result-row">
            <span className="fin-result-label">Signer key</span>
            <code className="fin-result-code">
              {result.verify_key ? result.verify_key.slice(0, 32) + '…' : '—'}
            </code>
          </div>
          <div className="fin-result-row fin-result-row--sig">
            <span className="fin-result-label">Signature</span>
            <code className="fin-result-code">
              {result.signature ? result.signature.slice(0, 48) + '…' : '—'}
            </code>
          </div>
          {verified !== null && (
            <div className={`fin-verify-result${verified ? ' fin-verify-ok' : ' fin-verify-fail'}`}>
              {verified
                ? '✓ Signature valid — instruction was not tampered with'
                : '✗ Signature invalid or instruction was modified'}
            </div>
          )}
        </div>
      )}
    </div>
  );
}

// ── Main ──────────────────────────────────────────────────────────────────────

export default function FinanceDashboard({ apiKey }) {
  const [activeUC, setActiveUC] = useState(null);

  return (
    <div>

      {/* ── Hero ── */}
      <div className="fin-hero">
        <div className="fin-hero-badge">Financial Services</div>
        <h2>
          Cryptographic Trust Infrastructure<br />
          for the AI Banking Era
        </h2>
        <p>
          Give every AI agent a verifiable identity. Sign every decision.
          Prove every consent. Detect rogue bots before they cause damage.
          All local, zero cloud, zero vendor lock-in.
        </p>
        <div className="fin-hero-stats">
          <div className="fin-stat">
            <span className="fin-stat-val">Ed25519</span>
            <span className="fin-stat-lbl">Same algorithm as TLS 1.3 &amp; modern HSMs</span>
          </div>
          <div className="fin-stat-div" />
          <div className="fin-stat">
            <span className="fin-stat-val">Local-first</span>
            <span className="fin-stat-lbl">Runs fully on-prem, no cloud dependency</span>
          </div>
          <div className="fin-stat-div" />
          <div className="fin-stat">
            <span className="fin-stat-val">6 regulations</span>
            <span className="fin-stat-lbl">SOX · FINRA · MiFID II · PSD2 · GDPR · DORA</span>
          </div>
        </div>
      </div>

      {/* ── Market gaps ── */}
      <div className="card">
        <h2>5 Gaps in Financial AI — That HSIP Fills Today</h2>
        <p className="fin-section-desc">
          These are real, unmet needs in production financial systems right now.
          Click any gap to see the exact problem and how HSIP addresses it.
        </p>
        <div className="fin-gaps">
          {MARKET_GAPS.map((g, i) => (
            <GapCard key={i} gap={g} index={i} />
          ))}
        </div>
      </div>

      {/* ── Use cases ── */}
      <div className="card">
        <h2>Use Cases</h2>
        <p className="fin-section-desc">
          Click a use case to see the body, the API surface, and the compliance mapping.
        </p>
        <div className="fin-uc-grid">
          {USE_CASES.map((uc, i) => (
            <UseCaseCard
              key={i}
              uc={uc}
              active={activeUC === i}
              onToggle={() => setActiveUC(activeUC === i ? null : i)}
            />
          ))}
        </div>
      </div>

      {/* ── Regulation coverage ── */}
      <div className="card">
        <h2>Regulation Coverage</h2>
        <div className="fin-reg-grid">
          {REGULATIONS.map((r, i) => (
            <RegBadge key={i} reg={r} />
          ))}
        </div>
        <div className="fin-reg-legend">
          <span><span className="fin-reg-dot" style={{ background: '#22c55e' }} /> Covered</span>
          <span><span className="fin-reg-dot" style={{ background: '#eab308' }} /> Partial</span>
          <span><span className="fin-reg-dot" style={{ background: '#818cf8' }} /> Roadmap</span>
        </div>
      </div>

      {/* ── Live demo ── */}
      <LiveSignDemo apiKey={apiKey} />

      {/* ── Comparison ── */}
      <div className="card">
        <h2>HSIP vs. Alternatives</h2>
        <ComparisonTable />
      </div>

    </div>
  );
}
