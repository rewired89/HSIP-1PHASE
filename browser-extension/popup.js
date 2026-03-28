// HSIP Browser Extension — Popup script

const HSIP_DASHBOARD = "http://127.0.0.1:7777";

async function getCurrentTabId() {
  const [tab] = await chrome.tabs.query({ active: true, currentWindow: true });
  return tab?.id ?? null;
}

async function loadStats(tabId) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage({ type: "GET_TAB_STATS", tabId }, (resp) => {
      resolve(resp ?? { count: 0, domains: [] });
    });
  });
}

async function getHsipStatus() {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage({ type: "GET_HSIP_STATUS" }, (resp) => {
      resolve(resp ?? { hasKey: false, connected: false });
    });
  });
}

async function saveApiKey(key) {
  return new Promise((resolve) => {
    chrome.runtime.sendMessage({ type: "SAVE_API_KEY", key }, (resp) => {
      resolve(resp ?? { connected: false });
    });
  });
}

function renderCount(count) {
  const el = document.getElementById("blockedCount");
  el.textContent = count;
  if (count > 0) {
    el.classList.remove("zero");
  } else {
    el.classList.add("zero");
  }
}

function renderDomains(domains) {
  const section = document.getElementById("domainSection");
  const list = document.getElementById("domainList");

  if (!domains || domains.length === 0) {
    section.style.display = "none";
    return;
  }

  section.style.display = "block";
  list.innerHTML = "";

  for (const domain of domains) {
    const item = document.createElement("div");
    item.className = "domain-item";
    item.innerHTML = `
      <div class="domain-dot"></div>
      <span class="domain-name">${escapeHtml(domain)}</span>
    `;
    list.appendChild(item);
  }
}

function renderHsipStatus(hasKey, connected) {
  const dot = document.getElementById("statusDot");
  const text = document.getElementById("hsipStatusText");
  const keySection = document.getElementById("keySection");

  if (connected) {
    dot.className = "status-dot connected";
    text.className = "hsip-status-text connected";
    text.textContent = "Connected";
    keySection.style.display = "none";
  } else if (hasKey) {
    dot.className = "status-dot disconnected";
    text.className = "hsip-status-text disconnected";
    text.textContent = "Not running";
    keySection.style.display = "none";
  } else {
    dot.className = "status-dot disconnected";
    text.className = "hsip-status-text disconnected";
    text.textContent = "Not connected";
    keySection.style.display = "block";
  }
}

function escapeHtml(str) {
  return str.replace(/&/g, "&amp;").replace(/</g, "&lt;").replace(/>/g, "&gt;");
}

// Wire up buttons
document.getElementById("saveKeyBtn").addEventListener("click", async () => {
  const key = document.getElementById("keyInput").value.trim();
  if (!key) return;
  const { connected } = await saveApiKey(key);
  const { hasKey } = await getHsipStatus();
  renderHsipStatus(hasKey, connected);
});

document.getElementById("keyInput").addEventListener("keydown", (e) => {
  if (e.key === "Enter") document.getElementById("saveKeyBtn").click();
});

document.getElementById("openDashboardBtn").addEventListener("click", () => {
  chrome.tabs.create({ url: HSIP_DASHBOARD });
});

// Init
(async () => {
  const tabId = await getCurrentTabId();
  if (tabId) {
    const { count, domains } = await loadStats(tabId);
    renderCount(count);
    renderDomains(domains);
  }

  const { hasKey, connected } = await getHsipStatus();
  renderHsipStatus(hasKey, connected);
})();
