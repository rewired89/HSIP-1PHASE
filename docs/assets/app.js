/* ============================================================
   HSIP Landing Page — JavaScript
   ============================================================ */

const RELEASES = "https://github.com/rewired89/HSIP-1PHASE/releases/latest";
const ASSETS = {
  windows: `${RELEASES}/download/hsip-windows-x64.exe`,
  macos_arm: `${RELEASES}/download/hsip-macos-arm64`,
  macos_x64: `${RELEASES}/download/hsip-macos-x64`,
  linux: `${RELEASES}/download/hsip-linux-x64`,
};

/* ── OS Detection ─────────────────────────────────────── */
function detectOS() {
  const ua = navigator.userAgent;
  if (/Windows/i.test(ua)) return "windows";
  if (/Mac/i.test(ua)) {
    // Rough Apple Silicon detection
    return /iPhone|iPad/.test(ua) || (navigator.maxTouchPoints > 1) ? "macos_arm" : "macos_x64";
  }
  if (/Linux/i.test(ua)) return "linux";
  return "windows"; // safe default
}

function osLabel(os) {
  return { windows: "Windows", macos_arm: "macOS (Apple Silicon)", macos_x64: "macOS (Intel)", linux: "Linux" }[os] || "Windows";
}

function osIcon(os) {
  if (os === "windows") return "🪟";
  if (os.startsWith("macos")) return "🍎";
  return "🐧";
}

/* ── Primary CTA ──────────────────────────────────────── */
function initPrimaryCTA() {
  const os = detectOS();
  const url = ASSETS[os];

  const btn = document.getElementById("btn-download-primary");
  const note = document.getElementById("os-detect-note");

  if (btn) {
    btn.href = url;
    btn.innerHTML = `<svg width="18" height="18" viewBox="0 0 24 24" fill="none" stroke="currentColor" stroke-width="2.5" stroke-linecap="round" stroke-linejoin="round"><path d="M21 15v4a2 2 0 0 1-2 2H5a2 2 0 0 1-2-2v-4"/><polyline points="7 10 12 15 17 10"/><line x1="12" y1="15" x2="12" y2="3"/></svg>
      Download for ${osLabel(os)}`;
  }

  if (note) {
    note.innerHTML = `${osIcon(os)} Detected <strong>${osLabel(os)}</strong> — <a href="#download">other platforms ↓</a>`;
  }

  // Mark detected card as recommended
  const cards = document.querySelectorAll("[data-os]");
  cards.forEach(card => {
    if (card.dataset.os === os || (os === "macos_arm" && card.dataset.os === "macos")) {
      card.classList.add("recommended");
    }
  });
}

/* ── FAQ accordion ────────────────────────────────────── */
function initFAQ() {
  document.querySelectorAll(".faq-item").forEach(item => {
    const btn = item.querySelector(".faq-q");
    if (!btn) return;
    btn.addEventListener("click", () => {
      const wasOpen = item.classList.contains("open");
      document.querySelectorAll(".faq-item.open").forEach(i => i.classList.remove("open"));
      if (!wasOpen) item.classList.add("open");
    });
  });
}

/* ── Copy CLI commands ────────────────────────────────── */
function initCopyCLI() {
  document.querySelectorAll(".cli-copy-btn").forEach(btn => {
    btn.addEventListener("click", () => {
      const target = document.getElementById(btn.dataset.target);
      if (!target) return;
      navigator.clipboard.writeText(target.textContent.trim()).then(() => {
        const orig = btn.textContent;
        btn.textContent = "Copied!";
        setTimeout(() => (btn.textContent = orig), 1800);
      });
    });
  });
}

/* ── Smooth active nav ────────────────────────────────── */
function initNav() {
  const sections = document.querySelectorAll("section[id]");
  const navLinks = document.querySelectorAll("nav a[href^='#']");

  const obs = new IntersectionObserver(entries => {
    entries.forEach(e => {
      if (e.isIntersecting) {
        navLinks.forEach(l => {
          l.style.color = l.getAttribute("href") === `#${e.target.id}` ? "var(--white)" : "";
        });
      }
    });
  }, { threshold: 0.4 });

  sections.forEach(s => obs.observe(s));
}

/* ── Typewriter for hero stat ─────────────────────────── */
function initTypewriter() {
  const el = document.getElementById("hero-stat");
  if (!el) return;
  const phrases = [
    "No cloud. No subscription.",
    "Your keys. Your machine.",
    "Open source. Fully auditable.",
    "Zero trackers leave your PC.",
  ];
  let i = 0, ci = 0, deleting = false;
  function tick() {
    const phrase = phrases[i % phrases.length];
    if (!deleting) {
      el.textContent = phrase.slice(0, ++ci);
      if (ci === phrase.length) { deleting = true; setTimeout(tick, 2200); return; }
    } else {
      el.textContent = phrase.slice(0, --ci);
      if (ci === 0) { deleting = false; i++; }
    }
    setTimeout(tick, deleting ? 35 : 60);
  }
  tick();
}

/* ── Init ─────────────────────────────────────────────── */
document.addEventListener("DOMContentLoaded", () => {
  initPrimaryCTA();
  initFAQ();
  initCopyCLI();
  initNav();
  initTypewriter();
});
