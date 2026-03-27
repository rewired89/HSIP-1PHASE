import React, { useState, useEffect, useCallback } from 'react';
import { request } from '../api';
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

// ── DNS Resolver section ──────────────────────────────────────────────────────

const DNS_PORT = 5300;

const DNS_OS_STEPS = {
  windows: [
    {
      n: 1,
      title: 'Open Network Settings',
      body: 'Start → Settings → Network & Internet → Change adapter options. Right-click your active adapter (Wi-Fi or Ethernet) → Properties.',
      code: null,
    },
    {
      n: 2,
      title: 'Change DNS to localhost',
      body: 'Select "Internet Protocol Version 4 (TCP/IPv4)" → Properties. Choose "Use the following DNS server addresses" and enter:',
      code: 'Preferred DNS: 127.0.0.1',
    },
    {
      n: 3,
      title: 'Redirect port 53 → 5300 (run once as Admin)',
      body: 'Open Command Prompt as Administrator and run:',
      code: `netsh interface portproxy add v4tov4 listenport=53 listenaddress=127.0.0.1 connectport=${DNS_PORT} connectaddress=127.0.0.1`,
    },
    {
      n: 4,
      title: 'Flush DNS and restart browser',
      body: 'Run the command below, then close and reopen your browser.',
      code: 'ipconfig /flushdns',
    },
  ],
  mac: [
    {
      n: 1,
      title: 'System Settings → Network',
      body: 'Open System Settings → Network → select your active connection → Details → DNS tab. Add 127.0.0.1 as the first DNS server.',
      code: null,
    },
    {
      n: 2,
      title: 'Redirect port 53 → 5300',
      body: 'Open Terminal and run (this persists until reboot):',
      code: `echo "rdr pass on lo0 proto udp from any to 127.0.0.1 port 53 -> 127.0.0.1 port ${DNS_PORT}" | sudo pfctl -ef -`,
    },
    {
      n: 3,
      title: 'Flush DNS cache',
      body: 'In Terminal:',
      code: 'sudo dscacheutil -flushcache && sudo killall -HUP mDNSResponder',
    },
    {
      n: 4,
      title: 'Restart your browser',
      body: 'Done — all tracker domains now resolve to NXDOMAIN, system-wide.',
      code: null,
    },
  ],
  linux: [
    {
      n: 1,
      title: 'Point systemd-resolved to localhost',
      body: 'Edit the resolved config:',
      code: 'sudo nano /etc/systemd/resolved.conf\n# Add under [Resolve]:\n# DNS=127.0.0.1\n# DNSStubListener=no',
    },
    {
      n: 2,
      title: 'Redirect port 53 → 5300',
      body: 'Add an iptables rule:',
      code: `sudo iptables -t nat -A OUTPUT -p udp --dport 53 -j REDIRECT --to-port ${DNS_PORT}`,
    },
    {
      n: 3,
      title: 'Restart resolver',
      body: '',
      code: 'sudo systemctl restart systemd-resolved',
    },
  ],
};

function DnsSection({ apiKey }) {
  const [status,    setStatus]    = useState(null);   // null = loading
  const [toggling,  setToggling]  = useState(false);
  const [dnsError,  setDnsError]  = useState('');
  const [log,       setLog]       = useState([]);
  const [dnsOs,     setDnsOs]     = useState('windows');
  const [showSetup, setShowSetup] = useState(false);

  const load = useCallback(async () => {
    try {
      const s = await request('GET', '/v1/dns/status', null, apiKey);
      setStatus(s);
      if (s.running) {
        const l = await request('GET', '/v1/dns/log', null, apiKey);
        setLog((l.entries || []).filter(e => e.blocked).slice(0, 10));
      }
    } catch { /* API not reachable or not authed */ }
  }, [apiKey]);

  useEffect(() => {
    load();
    const id = setInterval(load, 8000);
    return () => clearInterval(id);
  }, [load]);

  async function toggle() {
    if (!status) return;
    setToggling(true);
    setDnsError('');
    try {
      if (status.running) {
        const s = await request('POST', '/v1/dns/disable', null, apiKey);
        setStatus(s);
        setLog([]);
      } else {
        const s = await request('POST', '/v1/dns/enable', { port: DNS_PORT }, apiKey);
        setStatus(s);
      }
    } catch (e) { setDnsError(e.message); }
    setToggling(false);
  }

  if (!status) {
    return (
      <div className="card dns-card">
        <p className="empty">Loading DNS status…</p>
      </div>
    );
  }

  return (
    <div className={`card dns-card${status.running ? ' dns-card--active' : ''}`}>
      <div className="dns-header">
        <div className="dns-header-left">
          <div className="dns-icon">{status.running ? '🟢' : '⚪'}</div>
          <div>
            <h2 className="dns-title">Live DNS Blocker</h2>
            <p className="dns-subtitle">
              {status.running
                ? `Running on 127.0.0.1:${status.port} · blocking ${status.blocklist_size} tracker domains`
                : `Stops tracker connections before they load — system-wide, every app.`}
            </p>
          </div>
        </div>
        <button
          className={`dns-toggle-btn${status.running ? ' dns-toggle-btn--on' : ''}`}
          onClick={toggle}
          disabled={toggling}
        >
          {toggling ? '…' : status.running ? 'Turn Off' : 'Turn On'}
        </button>
      </div>

      {dnsError && (
        <div className="dns-error-banner">
          ⚠️ {dnsError}
        </div>
      )}

      {status.running && (
        <div className="dns-stats-row">
          <div className="dns-stat">
            <span className="dns-stat-num">{status.blocked_total.toLocaleString()}</span>
            <span className="dns-stat-label">blocked since start</span>
          </div>
          <div className="dns-stat">
            <span className="dns-stat-num">{status.queries_total.toLocaleString()}</span>
            <span className="dns-stat-label">total DNS queries</span>
          </div>
          <div className="dns-stat">
            <span className="dns-stat-num">{status.blocklist_size}</span>
            <span className="dns-stat-label">domains in blocklist</span>
          </div>
        </div>
      )}

      {status.running && log.length > 0 && (
        <div className="dns-log">
          <div className="dns-log-title">Recently blocked</div>
          {log.map((e, i) => (
            <div key={i} className="dns-log-entry">
              <span className="dns-log-domain">{e.domain}</span>
              {e.vendor && <span className="dns-log-vendor">{e.vendor}</span>}
            </div>
          ))}
        </div>
      )}

      {!status.running && (
        <div className="dns-explainer">
          <p>
            When turned on, HSIP runs a local DNS server on <code>127.0.0.1:{DNS_PORT}</code>.
            Every time your computer looks up a website, HSIP checks it against {status.blocklist_size} known
            tracking domains. Matches get a dead end — the tracker never loads.
          </p>
          <p>
            Unlike the hosts file below, this catches <strong>every app on your system</strong> in real time —
            no file editing, no rebooting required.
          </p>
        </div>
      )}

      {status.running && (
        <div style={{ marginTop: '1rem' }}>
          <button
            className="consumer-reset-btn"
            onClick={() => setShowSetup(v => !v)}
          >
            {showSetup ? '▲ Hide system DNS setup' : '⚙ Point your system DNS here'}
          </button>
          {showSetup && (
            <div className="dns-setup-panel">
              <p className="dns-setup-note">
                HSIP is running, but your system still uses a different DNS server.
                Follow these steps to route all DNS through HSIP.
              </p>
              <div className="os-picker" style={{ marginBottom: '1rem' }}>
                {[
                  { id: 'windows', label: '🪟 Windows' },
                  { id: 'mac',     label: '🍎 Mac' },
                  { id: 'linux',   label: '🐧 Linux' },
                ].map(o => (
                  <button
                    key={o.id}
                    className={`os-btn${dnsOs === o.id ? ' active' : ''}`}
                    onClick={() => setDnsOs(o.id)}
                  >
                    {o.label}
                  </button>
                ))}
              </div>
              <div className="setup-steps">
                {DNS_OS_STEPS[dnsOs].map(step => (
                  <DnsStepCard key={step.n} step={step} />
                ))}
              </div>
            </div>
          )}
        </div>
      )}
    </div>
  );
}

function DnsStepCard({ step }) {
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
        {step.body && <p>{step.body}</p>}
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

export default function ProtectionSetup({ apiKey }) {
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

      {/* Live DNS Blocker — requires apiKey */}
      {apiKey && <DnsSection apiKey={apiKey} />}

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
        <h3>What neither of these covers</h3>
        <p className="explainer-body">
          Both the DNS blocker and the hosts file stop <em>outbound connections</em> to
          known tracking domains. Neither prevents first-party tracking (a website
          tracking you on its own domain) or deep packet inspection by your ISP.
          For those threats, HSIP's coming HTTP/HTTPS proxy layer intercepts traffic
          at the connection level and shows you exactly what was blocked and why —
          that's Phase 2.
        </p>
      </div>
    </div>
  );
}
