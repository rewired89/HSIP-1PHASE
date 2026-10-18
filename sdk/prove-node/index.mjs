// HSIP client for Node 18+. No dependencies.
//
//   import { Prove } from './index.mjs'
//   const prove = new Prove('https://prove.example.com', process.env.HSIP_PROVE_KEY)
//   const { proof_id } = await prove.record({ application: '48211' }, {
//     stream: 'loan-decisions', actor: 'underwriting-model-v3', action: 'loan.decision',
//   })
//
// The record's content never leaves your process: only its SHA-256 is sent.

import { createHash, createPrivateKey, sign as edSign } from 'node:crypto'

export class ProveError extends Error {
  constructor(status, code, message) {
    super(`${status} ${code}: ${message}`)
    this.status = status
    this.code = code
  }
}

function canonical(value) {
  if (value === null || typeof value !== 'object') return JSON.stringify(value)
  if (Array.isArray(value)) return `[${value.map(canonical).join(',')}]`
  return `{${Object.keys(value).sort().map((k) => `${JSON.stringify(k)}:${canonical(value[k])}`).join(',')}}`
}

/** SHA-256 hex. Buffers and strings are hashed as-is; objects as key-sorted compact JSON. */
export function hashPayload(record) {
  const data = Buffer.isBuffer(record) || typeof record === 'string' ? record : canonical(record)
  return createHash('sha256').update(data).digest('hex')
}

// Customer-held keys: pass `signingKey` (the private key from `hsip-proof keygen`,
// as the JSON text or the base64 seed) and records are signed here, so the
// platform never holds your key.
const PKCS8_ED25519_PREFIX = Buffer.from('302e020100300506032b657004220420', 'hex')

function loadSigner(material) {
  let m = String(material).trim()
  if (m.startsWith('{')) m = JSON.parse(m).private_key_b64
  return createPrivateKey({ key: Buffer.concat([PKCS8_ED25519_PREFIX, Buffer.from(m, 'base64')]), format: 'der', type: 'pkcs8' })
}

/** RFC 8785 canonical JSON for the string-only envelope, then SHA-256 hex. */
export function eventHash(envelope) {
  return createHash('sha256').update(canonical(envelope)).digest('hex')
}

export class Prove {
  constructor(baseUrl, apiKey, { timeoutMs = 15000, signingKey } = {}) {
    this.baseUrl = baseUrl.replace(/\/+$/, '')
    this.apiKey = apiKey
    this.timeoutMs = timeoutMs
    this.signer = signingKey ? loadSigner(signingKey) : null
  }

  async #request(method, path, body, auth = true) {
    const headers = { 'Content-Type': 'application/json' }
    if (auth && this.apiKey) headers.Authorization = `Bearer ${this.apiKey}`
    const res = await fetch(this.baseUrl + path, {
      method,
      headers,
      body: body === undefined ? undefined : JSON.stringify(body),
      signal: AbortSignal.timeout(this.timeoutMs),
    })
    const data = await res.json().catch(() => null)
    if (!res.ok) throw new ProveError(res.status, data?.error?.code ?? 'error', data?.error?.message ?? res.statusText)
    return data
  }

  #item(record, { actor, action, subject = '', metadata = {}, payloadHash, stream }) {
    const payload_hash = payloadHash ?? hashPayload(record)
    return { ...(stream ? { stream } : {}), actor, action, subject, payload_hash, metadata }
  }

  /** Record one proof. */
  async record(record, opts) {
    const item = this.#item(record, opts)
    if (!this.signer) return this.#request('POST', '/v1/ingest', item)
    for (let attempt = 0; ; attempt++) {
      const prep = await this.#request('POST', '/v1/ingest/prepare', item)
      const env = prep.envelope
      if (eventHash(env) !== String(prep.event_hash).toLowerCase()) throw new ProveError(0, 'tampered', "the platform's event_hash does not match its envelope")
      for (const k of ['actor', 'action', 'subject', 'payload_hash']) {
        if (env[k] !== item[k]) throw new ProveError(0, 'tampered', `the platform changed \`${k}\``)
      }
      if (canonical(env.metadata) !== canonical(item.metadata)) throw new ProveError(0, 'tampered', 'the platform changed `metadata`')
      const signature = edSign(null, Buffer.from(prep.event_hash, 'hex'), this.signer).toString('base64')
      try {
        return await this.#request('POST', '/v1/ingest/commit', { ...(opts.stream ? { stream: opts.stream } : {}), envelope: env, signature })
      } catch (e) {
        if (e.status !== 409 || attempt >= 2) throw e // another writer moved the chain: prepare again
      }
    }
  }

  /** Record up to 100 items in order: [{ record, actor, action, ... }]. */
  async recordBatch(items, { stream } = {}) {
    if (this.signer) {
      const results = []
      for (const i of items) {
        try {
          const r = await this.record(i.record, { ...i, stream })
          results.push({ ok: true, proof_id: r.proof_id, seq: r.seq, event_hash: r.event_hash })
        } catch (e) {
          results.push({ ok: false, error: e.message })
          break
        }
      }
      return results
    }
    const out = await this.#request('POST', '/v1/ingest/batch', {
      ...(stream ? { stream } : {}),
      items: items.map((i) => this.#item(i.record, { ...i, stream: undefined })),
    })
    return out.results
  }

  /** Fetch a bundle with a `read` key. */
  getProof(proofId) {
    return this.#request('GET', `/v1/proofs/${proofId}`)
  }

  /** Server-side verification. For independent assurance run `hsip-proof verify` offline. */
  verify(bundleOrPack) {
    return this.#request('POST', '/v1/verify', bundleOrPack, false)
  }
}
