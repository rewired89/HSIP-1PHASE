// HSIP Browser Extension — Background Service Worker
// Tracks blocked requests per tab and syncs with the local HSIP API.

const HSIP_API = "http://127.0.0.1:7777";

// Per-tab blocked count: { tabId -> { count, domains: Set } }
const tabStats = new Map();

// Load API key from storage
async function getApiKey() {
  const { hsipApiKey } = await chrome.storage.local.get("hsipApiKey");
  return hsipApiKey || null;
}

// Probe HSIP API availability and update connection status
async function checkHsipConnection() {
  try {
    const key = await getApiKey();
    if (!key) {
      await chrome.storage.local.set({ hsipConnected: false });
      return false;
    }
    const res = await fetch(`${HSIP_API}/health`, {
      headers: { Authorization: `Bearer ${key}` },
      signal: AbortSignal.timeout(2000),
    });
    const connected = res.ok;
    await chrome.storage.local.set({ hsipConnected: connected });
    return connected;
  } catch {
    await chrome.storage.local.set({ hsipConnected: false });
    return false;
  }
}

// Reset stats for a tab when it navigates
chrome.tabs.onUpdated.addListener((tabId, changeInfo) => {
  if (changeInfo.status === "loading") {
    tabStats.set(tabId, { count: 0, domains: new Set() });
    chrome.action.setBadgeText({ tabId, text: "" });
  }
});

chrome.tabs.onRemoved.addListener((tabId) => {
  tabStats.delete(tabId);
});

// Listen for blocked requests via declarativeNetRequestFeedback
// Chrome fires onRuleMatchedDebug only in dev; use webRequest for counting in production.
// We intercept via content script messages instead (see content.js).
chrome.runtime.onMessage.addListener((msg, sender, sendResponse) => {
  if (msg.type === "TRACKER_BLOCKED") {
    const tabId = sender.tab?.id;
    if (!tabId) return;

    let stats = tabStats.get(tabId) || { count: 0, domains: new Set() };
    stats.count += msg.count ?? 1;
    if (msg.domain) stats.domains.add(msg.domain);
    tabStats.set(tabId, stats);

    // Update badge
    const label = stats.count > 99 ? "99+" : String(stats.count);
    chrome.action.setBadgeText({ tabId, text: label });
    chrome.action.setBadgeBackgroundColor({ tabId, color: "#e53e3e" });

    sendResponse({ ok: true });
    return true;
  }

  if (msg.type === "GET_TAB_STATS") {
    const tabId = msg.tabId;
    const stats = tabStats.get(tabId) || { count: 0, domains: new Set() };
    sendResponse({
      count: stats.count,
      domains: Array.from(stats.domains),
    });
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
    chrome.storage.local.get(["hsipApiKey", "hsipConnected"]).then((data) => {
      sendResponse({
        hasKey: !!data.hsipApiKey,
        connected: !!data.hsipConnected,
      });
    });
    return true;
  }
});

// Periodically verify HSIP connection (every 30s)
chrome.alarms.create("hsip_heartbeat", { periodInMinutes: 0.5 });
chrome.alarms.onAlarm.addListener((alarm) => {
  if (alarm.name === "hsip_heartbeat") {
    checkHsipConnection();
  }
});

// Check on startup
checkHsipConnection();
