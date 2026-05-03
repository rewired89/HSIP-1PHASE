// HSIP Browser Extension — Background Service Worker
// Tracks blocked requests per tab and syncs with the local HSIP API.

const HSIP_API = "http://127.0.0.1:7474";

// Per-tab blocked count: { tabId -> { count, domains: Set } }
const tabStats = new Map();

async function getApiKey() {
  const { hsipApiKey } = await chrome.storage.local.get("hsipApiKey");
  return hsipApiKey || null;
}

// Probe HSIP API and fetch fresh agent activity in one pass
async function checkHsipConnection() {
  try {
    const key = await getApiKey();
    if (!key) {
      await chrome.storage.local.set({ hsipConnected: false, hsipActivity: [] });
      return false;
    }

    const res = await fetch(`${HSIP_API}/health`, {
      signal: AbortSignal.timeout(2000),
    });

    if (!res.ok) {
      await chrome.storage.local.set({ hsipConnected: false, hsipActivity: [] });
      return false;
    }

    await chrome.storage.local.set({ hsipConnected: true });

    // Fetch recent agent activity while we have a live connection
    fetchAgentActivity(key);
    return true;
  } catch {
    await chrome.storage.local.set({ hsipConnected: false, hsipActivity: [] });
    return false;
  }
}

async function fetchAgentActivity(key) {
  try {
    const res = await fetch(`${HSIP_API}/v1/audit?limit=5`, {
      headers: { Authorization: `Bearer ${key}` },
      signal: AbortSignal.timeout(3000),
    });
    if (!res.ok) return;
    const entries = await res.json();
    await chrome.storage.local.set({ hsipActivity: entries });
  } catch {
    // Non-fatal — leave existing cached activity in place
  }
}

// ── Tab lifecycle ──────────────────────────────────────────────────────────────

chrome.tabs.onUpdated.addListener((tabId, changeInfo) => {
  if (changeInfo.status === "loading") {
    tabStats.set(tabId, { count: 0, domains: new Set() });
    chrome.action.setBadgeText({ tabId, text: "" });
  }
});

chrome.tabs.onRemoved.addListener((tabId) => {
  tabStats.delete(tabId);
});

// ── Message handlers ───────────────────────────────────────────────────────────

chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.type === "TRACKER_BLOCKED") {
    const tabId = sender.tab?.id;
    if (!tabId) return;

    let stats = tabStats.get(tabId) || { count: 0, domains: new Set() };
    stats.count += msg.count ?? 1;
    if (msg.domain) stats.domains.add(msg.domain);
    tabStats.set(tabId, stats);

    const label = stats.count > 99 ? "99+" : String(stats.count);
    chrome.action.setBadgeText({ tabId, text: label });
    chrome.action.setBadgeBackgroundColor({ tabId, color: "#e53e3e" });

    sendResponse({ ok: true });
    return true;
  }

  if (msg.type === "GET_TAB_STATS") {
    const stats = tabStats.get(msg.tabId) || { count: 0, domains: new Set() };
    sendResponse({ count: stats.count, domains: Array.from(stats.domains) });
    return true;
  }

  if (msg.type === "CHECK_HSIP") {
    checkHsipConnection().then((connected) => sendResponse({ connected }));
    return true;
  }

  if (msg.type === "SAVE_API_KEY") {
    chrome.storage.local.set({ hsipApiKey: msg.key }).then(() => {
      checkHsipConnection().then((connected) => sendResponse({ connected }));
    });
    return true;
  }

  if (msg.type === "GET_HSIP_STATUS") {
    chrome.storage.local
      .get(["hsipApiKey", "hsipConnected", "hsipActivity"])
      .then((data) => {
        sendResponse({
          hasKey: !!data.hsipApiKey,
          connected: !!data.hsipConnected,
          activity: data.hsipActivity || [],
        });
      });
    return true;
  }
});

// ── Heartbeat (every 30 s) ─────────────────────────────────────────────────────

chrome.alarms.create("hsip_heartbeat", { periodInMinutes: 0.5 });
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === "hsip_heartbeat") checkHsipConnection();
});

checkHsipConnection();
