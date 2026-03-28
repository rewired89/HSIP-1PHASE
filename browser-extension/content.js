// HSIP Browser Extension — Content Script
// Detects tracker domains that were blocked on this page and reports counts
// to the background service worker.
//
// How it works:
// The declarativeNetRequest rules block tracker requests silently.
// This script inspects the page's resource timing entries to count
// requests that were attempted (and therefore blocked).

"use strict";

// Tracker domains — must stay in sync with rules.json
const TRACKER_DOMAINS = new Set([
  "hotjar.com", "fullstory.com", "clarity.ms", "logrocket.com", "mouseflow.com",
  "luckyorange.com", "inspectlet.com", "smartlook.com", "doubleclick.net",
  "googlesyndication.com", "criteo.com", "adnxs.com", "adsrvr.org",
  "amazon-adsystem.com", "outbrain.com", "taboola.com", "rubiconproject.com",
  "openx.net", "pubmatic.com", "casalemedia.com", "sharethis.com",
  "facebook.com", "fbcdn.net", "tiktok.com", "linkedin.com",
  "snapchat.com", "ads-twitter.com", "pinterest.com", "google-analytics.com",
  "googletagmanager.com", "mixpanel.com", "amplitude.com", "segment.io",
  "cdn.segment.com", "heapanalytics.com", "intercom.io", "hubspot.com",
  "marketo.com", "pardot.com", "woopra.com", "chartbeat.com",
  "getclicky.com", "statcounter.com", "vortex.data.microsoft.com",
  "settings-win.data.microsoft.com", "applicationinsights.io", "bat.bing.com",
  "scorecardresearch.com", "quantserve.com", "imrworldwide.com",
  "bluekai.com", "crwdcntrl.net", "list-manage.com", "sendgrid.net",
  "cloudflareinsights.com", "fpjs.io", "online-metrix.net", "sentry.io",
  "crashlyticsreports-pa.googleapis.com", "datadoghq-browser-agent.com",
  "adservice.google.com",
]);

// Match a hostname against the blocked domains list
function isTrackerDomain(hostname) {
  if (TRACKER_DOMAINS.has(hostname)) return hostname;
  // Check parent domains (e.g. "cdn.hotjar.com" → "hotjar.com")
  const parts = hostname.split(".");
  for (let i = 1; i < parts.length - 1; i++) {
    const candidate = parts.slice(i).join(".");
    if (TRACKER_DOMAINS.has(candidate)) return candidate;
  }
  return null;
}

// Analyse PerformanceResourceTiming entries:
// Blocked requests have transferSize=0 and decodedBodySize=0 but an initiator.
// We look for entries pointing at tracker domains.
function scanResourceTimings() {
  const entries = performance.getEntriesByType("resource");
  const found = new Map(); // domain -> count

  for (const entry of entries) {
    try {
      const url = new URL(entry.name);
      const match = isTrackerDomain(url.hostname);
      if (match) {
        found.set(match, (found.get(match) || 0) + 1);
      }
    } catch {
      // Ignore unparseable URLs
    }
  }

  return found;
}

// Report blocked trackers to the background worker
function reportBlocked(domainMap) {
  if (domainMap.size === 0) return;

  for (const [domain, count] of domainMap) {
    chrome.runtime.sendMessage({
      type: "TRACKER_BLOCKED",
      domain,
      count,
    });
  }
}

// Initial scan once the page has loaded
function runScan() {
  const blocked = scanResourceTimings();
  reportBlocked(blocked);
}

// Run after page load
if (document.readyState === "complete") {
  runScan();
} else {
  window.addEventListener("load", runScan, { once: true });
}

// Also observe dynamically injected resources (SPAs, lazy loads)
if (typeof PerformanceObserver !== "undefined") {
  const observer = new PerformanceObserver((list) => {
    const found = new Map();
    for (const entry of list.getEntries()) {
      try {
        const url = new URL(entry.name);
        const match = isTrackerDomain(url.hostname);
        if (match) {
          found.set(match, (found.get(match) || 0) + 1);
        }
      } catch {
        // ignore
      }
    }
    reportBlocked(found);
  });

  observer.observe({ type: "resource", buffered: false });
}
