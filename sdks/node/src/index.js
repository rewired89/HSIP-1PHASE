'use strict';

const https  = require('https');
const http   = require('http');
const url    = require('url');
const crypto = require('crypto');
const fs     = require('fs');
const path   = require('path');

class HSIPError extends Error {
  constructor(message, statusCode) {
    super(message);
    this.name       = 'HSIPError';
    this.statusCode = statusCode;
  }
}

class HSIPClient {
  /**
   * HSIP REST API client.
   * @param {Object} opts
   * @param {string} opts.apiKey    - Bearer token (hsip_...)
   * @param {string} [opts.baseUrl] - API base URL (default: http://localhost:3000)
   */
  constructor({ apiKey, baseUrl = 'http://localhost:3000' }) {
    this.apiKey  = apiKey;
    this.baseUrl = baseUrl.replace(/\/$/, '');
  }

  _request(method, path, body) {
    return new Promise((resolve, reject) => {
      const parsed  = new url.URL(this.baseUrl + path);
      const lib     = parsed.protocol === 'https:' ? https : http;
      const payload = body ? JSON.stringify(body) : null;

      const opts = {
        hostname: parsed.hostname,
        port:     parsed.port || (parsed.protocol === 'https:' ? 443 : 80),
        path:     parsed.pathname + parsed.search,
        method,
        headers: {
          'Authorization': `Bearer ${this.apiKey}`,
          'Content-Type':  'application/json',
          ...(payload ? { 'Content-Length': Buffer.byteLength(payload) } : {}),
        },
      };

      const req = lib.request(opts, (res) => {
        let data = '';
        res.on('data', (chunk) => data += chunk);
        res.on('end', () => {
          try {
            const parsed = JSON.parse(data);
            if (res.statusCode >= 400) {
              reject(new HSIPError(parsed.error || data, res.statusCode));
            } else {
              resolve(parsed);
            }
          } catch (e) {
            reject(new HSIPError(`Failed to parse response: ${data}`));
          }
        });
      });

      req.on('error', reject);
      if (payload) req.write(payload);
      req.end();
    });
  }

  // Identity
  getOrCreateIdentity()  { return this._request('POST', '/v1/identity'); }
  getIdentity()          { return this._request('GET',  '/v1/identity'); }

  // Consent
  grantConsent(peerVerifyKey, ttlMs = 3_600_000) {
    return this._request('POST', '/v1/consent/grant', { peer_verify_key: peerVerifyKey, ttl_ms: ttlMs });
  }
  revokeConsent(peerVerifyKey) {
    return this._request('POST', '/v1/consent/revoke', { peer_verify_key: peerVerifyKey });
  }
  listConsents()                  { return this._request('GET', '/v1/consent'); }
  getConsent(peerVerifyKey)       { return this._request('GET', `/v1/consent/${peerVerifyKey}`); }

  // Messages
  signMessage(content, peerVerifyKey = null) {
    const body = { content };
    if (peerVerifyKey) body.peer_verify_key = peerVerifyKey;
    return this._request('POST', '/v1/messages/sign', body);
  }
  verifyMessage(content, signature, peerVerifyKey) {
    return this._request('POST', '/v1/messages/verify', {
      content, signature, peer_verify_key: peerVerifyKey,
    });
  }
  listMessages() { return this._request('GET', '/v1/messages'); }

  // Audit
  getAuditLog(limit = 50, action = null) {
    let path = `/v1/audit?limit=${limit}`;
    if (action) path += `&action=${action}`;
    return this._request('GET', path);
  }

  // API Keys
  createKey(name = 'default', agentType = 'human') {
    return this._request('POST', '/v1/keys', { name, agent_type: agentType });
  }
  listKeys()       { return this._request('GET', '/v1/keys'); }
  revokeKey(keyId) { return this._request('DELETE', `/v1/keys/${keyId}`); }

  // AI Agent governance

  /**
   * Register an AI agent and receive its API key (shown once).
   * @param {string} name
   * @param {number|null} [expiresDays]
   * @returns {Promise<{id:string, key:string, name:string, agent_type:string, created_at:number, expires_at:number|null}>}
   */
  registerAgent(name, expiresDays = null) {
    const body = { name, agent_type: 'ai_agent' };
    if (expiresDays !== null) body.expires_in_days = expiresDays;
    return this._request('POST', '/v1/keys', body);
  }

  /** List AI agents with live velocity stats. */
  listAgents() { return this._request('GET', '/v1/agents'); }

  /** Immediately revoke an AI agent's access by key ID. */
  revokeAgent(keyId) { return this._request('DELETE', `/v1/keys/${keyId}`); }

  /**
   * Write a signed, tamper-proof action record to the audit log.
   * @param {string} action  - Short verb, e.g. "file.read", "email.send"
   * @param {string|null} [detail]
   */
  logAction(action, detail = null) {
    const content = detail ? `[ACTION:${action}] ${detail}` : `[ACTION:${action}]`;
    return this._request('POST', '/v1/messages/sign', { content });
  }

  /** Probe localhost for running AI agents / MCP servers. */
  discoverAgents() { return this._request('GET', '/v1/agents/discover'); }

  // ── Decision attestations ─────────────────────────────────────────────
  //
  // HSIP never receives or stores the actual content of a decision (trade
  // parameters, etc.) — only its SHA-256 hash. Disclosure of the real
  // payload, if ever needed, happens entirely on your side.

  /**
   * Hex-encoded SHA-256 of a decision payload, ready for `payloadHash`.
   * @param {Buffer|string} payload
   * @returns {string}
   */
  static hashPayload(payload) {
    return crypto.createHash('sha256').update(payload).digest('hex');
  }

  /**
   * Sign and chain one AI-agent decision attestation.
   *
   * `payloadHash` must be the hex-encoded SHA-256 of your actual (never
   * disclosed to HSIP) decision content — see `HSIPClient.hashPayload`.
   *
   * Returns a self-contained receipt: { decision_id, envelope, event_hash,
   * signature, sign_algo, issuer_verify_key }. If `receiptDir` is given,
   * the receipt is also written to disk immediately — this is the
   * client-side mitigation for the gap between signing and anchoring: if
   * this HSIP instance's own database were ever tampered with or a
   * decision deleted before the next anchor cycle, your own copy is
   * independent proof the decision was signed.
   *
   * @param {Object} opts
   * @param {string} opts.accountableKey
   * @param {string} opts.modelVersion
   * @param {string} opts.strategyId
   * @param {string} opts.decisionType
   * @param {string} opts.payloadHash
   * @param {string} [opts.receiptDir]
   */
  async recordDecision({ accountableKey, modelVersion, strategyId, decisionType, payloadHash, receiptDir }) {
    const receipt = await this._request('POST', '/v1/decisions', {
      accountable_key: accountableKey,
      model_version:   modelVersion,
      strategy_id:     strategyId,
      decision_type:   decisionType,
      payload_hash:    payloadHash,
    });
    if (receiptDir) HSIPClient.saveReceipt(receipt, receiptDir);
    return receipt;
  }

  /**
   * Persist a decision receipt to `<receiptDir>/<decision_id>.json`.
   * Returns the path written. Safe to call independently of
   * `recordDecision` (e.g. to re-save a receipt fetched later).
   * @param {Object} receipt
   * @param {string} receiptDir
   * @returns {string}
   */
  static saveReceipt(receipt, receiptDir) {
    fs.mkdirSync(receiptDir, { recursive: true });
    const filePath = path.join(receiptDir, `${receipt.decision_id}.json`);
    fs.writeFileSync(filePath, JSON.stringify(receipt, null, 2));
    return filePath;
  }

  /** List this tenant's decision attestations, newest first. */
  listDecisions() { return this._request('GET', '/v1/decisions'); }

  /**
   * Full self-contained verification bundle for one decision. Before the
   * next anchor cycle runs, `anchored` is false and only authorship
   * (signature) is provable yet — call again later once a batch anchors.
   * @param {string} decisionId
   */
  getDecisionProof(decisionId) {
    return this._request('GET', `/v1/decisions/${decisionId}/proof`);
  }

  /**
   * Verify a decision proof bundle. This calls HSIP's
   * `/v1/decisions/verify` endpoint, but that endpoint takes no API key
   * and touches no database — it's a pure function of `bundle`, so any
   * party can run the equivalent check themselves without this SDK or an
   * HSIP account at all.
   * @param {Object} bundle
   */
  verifyDecision(bundle) {
    return this._request('POST', '/v1/decisions/verify', bundle);
  }
}

module.exports = { HSIPClient, HSIPError };
