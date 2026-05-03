'use strict';

const https = require('https');
const http  = require('http');
const url   = require('url');

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
}

module.exports = { HSIPClient, HSIPError };
