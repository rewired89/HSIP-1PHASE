# HSIP Browser Extension

Shows how many trackers HSIP is blocking on the current page — right in your browser toolbar.

## What it does

- Blocks 60+ tracker domains via Chrome's `declarativeNetRequest` API (no performance cost)
- Shows a live badge count of trackers blocked on the current page
- Lists exactly which tracker companies were stopped
- Connects to your local HSIP instance to show server status

## Install (Development / Unpacked)

1. Clone the repo and navigate to this folder
2. Open `chrome://extensions` in Chrome (or `edge://extensions` in Edge)
3. Enable **Developer mode** (top right toggle)
4. Click **Load unpacked** and select this `browser-extension/` folder
5. The HSIP shield icon appears in your toolbar

## Connect to HSIP

The extension works standalone (tracker blocking + counting) without HSIP running.

To also show HSIP server status:
1. Start HSIP: `hsip` (or however you run it)
2. Click the extension icon → enter your API key from `~/.hsip/admin.key`
3. The status dot turns green when HSIP is running

## Firefox

Firefox uses Manifest V3 with minor differences. To load in Firefox:
1. Open `about:debugging#/runtime/this-firefox`
2. Click **Load Temporary Add-on**
3. Select the `manifest.json` file in this folder

## Publishing to Chrome Web Store

1. Update `sha256` values in `Formula/hsip.rb` with real release hashes
2. Bump `version` in `manifest.json`
3. Zip the contents of this folder (not the folder itself)
4. Upload to [Chrome Web Store Developer Dashboard](https://chrome.google.com/webstore/devconsole)

## Files

```
browser-extension/
├── manifest.json       Chrome/Edge Manifest V3
├── background.js       Service worker — tracks blocked counts per tab
├── content.js          Content script — detects tracker requests via PerformanceObserver
├── popup.html          Extension popup UI
├── popup.js            Popup logic
├── rules.json          declarativeNetRequest blocklist (61 rules)
└── icons/
    ├── icon16.svg
    ├── icon48.svg
    └── icon128.svg
```
