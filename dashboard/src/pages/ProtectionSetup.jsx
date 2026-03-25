import React, { useState } from 'react';
import { TRACKERS, RISK_LEVEL } from '../data/trackers';

// Only include trackers that are explicitly safe to block
const BLOCKABLE = TRACKERS.filter(t => t.safeToBlock);
const CRITICAL_COUNT = BLOCKABLE.filter(t => t.risk === 'critical').length;
const HIGH_COUNT     = BLOCKABLE.filter(t => t.risk === 'high').length;

function buildHostsContent() {
  const date   = new Date().toLocaleDateString('en-US', { year: 'numeric', month: 'long', day: 'numeric' });
  const header = [
    '# ============================================================',
    '# HSIP Privacy Hosts File',
    `# Generated: ${date}`,
    `# Blocking ${BLOCKABLE.length} known tracking domains`,
    '# ============================================================',
    '#',
    '# HOW TO USE:',
    '#   Windows : open C:\\Windows\\System32\\drivers\\etc\\hosts',
    '#             as Administrator and paste these lines at the bottom.',
    '#   Mac     : sudo nano /etc/hosts  — paste at bottom, Ctrl+X to save.',
    '#   Linux   : sudo nano /etc/hosts  — paste at bottom, save.',
    '#',
    '# To undo: remove the lines between the HSIP markers below.',
    '#',
    '# ---- HSIP BLOCK START ----------------------------------------',
  ];

  const entries = BLOCKABLE.flatMap(t => {
    const domain = t.domain.replace(/^\*\./, '');
    return [
      `# ${t.vendor} — ${t.plain}`,
      `0.0.0.0 ${domain}`,
      `0.0.0.0 www.${domain}`,
      '',
    ];
  });

  const footer = ['# ---- HSIP BLOCK END ------------------------------------------'];

  return [...header, '', ...entries, ...footer].join('\n');
}

function downloadHosts() {
  const content = buildHostsContent();
  const blob    = new Blob([content], { type: 'text/plain' });
  const url     = URL.createObjectURL(blob);
  const a       = document.createElement('a');
  a.href        = url;
  a.download    = 'hsip-privacy-hosts.txt';
  a.click();
  URL.revokeObjectURL(url);
}

// ── Per-OS setup steps ────────────────────────────────────────────────────────

const OS_STEPS = {
  windows: [
    {
      n: 1,
      title: 'Download the hosts file',
      body: 'Click the blue "Download Hosts File" button below. Save it somewhere easy to find — your Desktop is fine.',
      code: null,
    },
    {
      n: 2,
      title: 'Open Notepad as Administrator',
      body: 'Click the Windows Start menu → type "Notepad" → right-click the result → choose "Run as administrator" → click Yes.',
      code: null,
    },
    {
      n: 3,
      title: 'Open your hosts file',
      body: 'In Notepad: File → Open. Navigate to the path below. Change the file type dropdown to "All Files (*.*)", then open the file named hosts.',
      code: 'C:\\Windows\\System32\\drivers\\etc\\hosts',
    },
    {
      n: 4,
      title: 'Paste the new entries',
      body: 'Open the file you downloaded in step 1 with any text editor. Select All (Ctrl+A) and Copy (Ctrl+C). Go back to the hosts file in Notepad, scroll to the very bottom, and Paste (Ctrl+V).',
      code: null,
    },
    {
      n: 5,
      title: 'Save and flush DNS',
      body: 'Save the hosts file in Notepad (Ctrl+S). Then open Command Prompt and run:',
      code: 'ipconfig /flushdns',
    },
    {
      n: 6,
      title: 'Restart your browser',
      body: 'Close and reopen Chrome, Edge, Firefox, or whichever browser you use. Done — tracking is now blocked across your entire computer, not just one browser.',
      code: null,
    },
  ],
  mac: [
    {
      n: 1,
      title: 'Download the hosts file',
      body: 'Click "Download Hosts File" below. It will land in your Downloads folder.',
      code: null,
    },
    {
      n: 2,
      title: 'Open Terminal',
      body: 'Press Cmd + Space, type "Terminal", and press Enter.',
      code: null,
    },
    {
      n: 3,
      title: 'Append the entries to your hosts file',
      body: 'In Terminal, run the command below. It will ask for your Mac password.',
      code: 'cat ~/Downloads/hsip-privacy-hosts.txt | sudo tee -a /etc/hosts',
    },
    {
      n: 4,
      title: 'Flush your DNS cache',
      body: 'Still in Terminal, run:',
      code: 'sudo dscacheutil -flushcache && sudo killall -HUP mDNSResponder',
    },
    {
      n: 5,
      title: 'Restart your browser',
      body: 'Close and reopen your browser. Done — all listed trackers are now blocked system-wide.',
      code: null,
    },
  ],
  linux: [
    {
      n: 1,
      title: 'Download the hosts file',
      body: 'Click "Download Hosts File" below.',
      code: null,
    },
    {
      n: 2,
      title: 'Append to your hosts file',
      body: 'Open a terminal and run:',
      code: 'cat ~/Downloads/hsip-privacy-hosts.txt | sudo tee -a /etc/hosts',
    },
    {
      n: 3,
      title: 'Flush your DNS cache',
      body: 'Run one of these depending on your distro:',
      code: 'sudo systemctl restart systemd-resolved\n# OR (older systems):\nsudo service networking restart',
    },
    {
      n: 4,
      title: 'Restart your browser',
      body: 'Close and reopen your browser. Protection is now active.',
      code: null,
    },
  ],
};

function StepCard({ step }) {
  const [copied, setCopied] = useState(false);

  function copyCode() {
    navigator.clipboard.writeText(step.code);
    setCopied(true);
    setTimeout(() => setCopied(false), 2000);
  }

  return (
    <div className="setup-step">
      <div className="setup-step-num">{step.n}</div>
      <div className="setup-step-body">
        <strong>{step.title}</strong>
        <p>{step.body}</p>
        {step.code && (
          <div className="setup-code-block">
            <pre>{step.code}</pre>
            <button className="setup-copy-btn" onClick={copyCode}>
              {copied ? '✓ Copied' : 'Copy'}
            </button>
          </div>
        )}
      </div>
    </div>
  );
}

export default function ProtectionSetup() {
  const [os,        setOs]        = useState('windows');
  const [downloaded, setDownloaded] = useState(false);

  function handleDownload() {
    downloadHosts();
    setDownloaded(true);
  }

  return (
    <div>
      <div className="consumer-hero">
        <div className="consumer-hero-icon">🔒</div>
        <h2>Enable Protection</h2>
        <p>
          Block {BLOCKABLE.length} known tracking companies on your entire computer — every app,
          every browser — in under 5 minutes. No software to install.
        </p>
      </div>

      {/* Score card */}
      <div className="card protection-score-card">
        <div className="ps-score-row">
          <div className="ps-score-item">
            <span className="ps-score-num ps-score-critical">{CRITICAL_COUNT}</span>
            <span className="ps-score-label">Critical trackers blocked</span>
          </div>
          <div className="ps-score-divider" />
          <div className="ps-score-item">
            <span className="ps-score-num ps-score-high">{HIGH_COUNT}</span>
            <span className="ps-score-label">High-risk trackers blocked</span>
          </div>
          <div className="ps-score-divider" />
          <div className="ps-score-item">
            <span className="ps-score-num ps-score-total">{BLOCKABLE.length}</span>
            <span className="ps-score-label">Total domains blocked</span>
          </div>
        </div>
        <div className="ps-includes">
          Includes: Google Analytics · Facebook Pixel · Hotjar · FullStory · TikTok ·
          DoubleClick · Criteo · Microsoft Clarity · Mixpanel · and more
        </div>
      </div>

      {/* How it works */}
      <div className="card">
        <h2>How does this work?</h2>
        <div className="ps-how-grid">
          <div className="ps-how-item">
            <span>📄</span>
            <div>
              <strong>The hosts file</strong>
              <p>
                Every computer has a file called "hosts" that maps domain names to IP
                addresses. By pointing tracker domains to 0.0.0.0 (nothing), your
                computer simply refuses to connect to them — before they can run.
              </p>
            </div>
          </div>
          <div className="ps-how-item">
            <span>🌐</span>
            <div>
              <strong>Works everywhere, not just one browser</strong>
              <p>
                Unlike a browser extension, the hosts file blocks trackers in Chrome,
                Firefox, Edge, Safari, and even inside desktop apps — all at once.
              </p>
            </div>
          </div>
          <div className="ps-how-item">
            <span>✅</span>
            <div>
              <strong>Safe to block</strong>
              <p>
                Every domain in this file was reviewed by HSIP and marked
                "safe to block" — removing them won't break websites you use,
                only their ability to track you.
              </p>
            </div>
          </div>
          <div className="ps-how-item">
            <span>↩️</span>
            <div>
              <strong>Easy to undo</strong>
              <p>
                The hosts file has clear HSIP markers. To remove the block, just
                delete the lines between <code># HSIP BLOCK START</code> and
                <code># HSIP BLOCK END</code>.
              </p>
            </div>
          </div>
        </div>
      </div>

      {/* Download + steps */}
      <div className="card">
        <h2>Set It Up</h2>

        {/* OS picker */}
        <div className="os-picker">
          {[
            { id: 'windows', label: '🪟 Windows' },
            { id: 'mac',     label: '🍎 Mac' },
            { id: 'linux',   label: '🐧 Linux' },
          ].map(o => (
            <button
              key={o.id}
              className={`os-btn${os === o.id ? ' active' : ''}`}
              onClick={() => setOs(o.id)}
            >
              {o.label}
            </button>
          ))}
        </div>

        {/* Download button */}
        <button
          className={`primary setup-download-btn${downloaded ? ' setup-download-done' : ''}`}
          onClick={handleDownload}
        >
          {downloaded
            ? '✓ Downloaded — follow the steps below'
            : '⬇ Download Hosts File'}
        </button>

        {/* Steps */}
        <div className="setup-steps">
          {OS_STEPS[os].map(step => (
            <StepCard key={step.n} step={step} />
          ))}
        </div>
      </div>

      {/* What gets blocked */}
      <div className="card">
        <h2>What gets blocked</h2>
        <p className="aiwatch-normal-note">
          These {BLOCKABLE.length} trackers will no longer be able to load on your computer.
        </p>
        <div className="blocked-list">
          {BLOCKABLE.map((t, i) => {
            const r = RISK_LEVEL[t.risk];
            return (
              <div key={i} className="blocked-item">
                <span
                  className="blocked-risk-dot"
                  style={{ background: r.color }}
                  title={r.label}
                />
                <span className="blocked-vendor">{t.vendor}</span>
                <span className="blocked-plain">{t.plain}</span>
              </div>
            );
          })}
        </div>
      </div>

      <div className="consumer-explainer card">
        <h3>What this doesn't cover</h3>
        <p className="explainer-body">
          The hosts file blocks connections to domains you've never consented to.
          It does <strong>not</strong> prevent first-party tracking (a website tracking
          your behavior on its own domain), HTTPS inspection, or tracking through
          shared infrastructure. For deeper protection, HSIP's telemetry guard
          can be integrated as a local proxy — that's the next phase of development.
        </p>
      </div>
    </div>
  );
}
