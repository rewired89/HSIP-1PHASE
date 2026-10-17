// HSIP Prove client for Node 18+. No dependencies.
//
//   import { Prove } from './index.mjs'
//   const prove = new Prove('https://prove.example.com', process.env.HSIP_PROVE_KEY)
//   const { proof_id } = await prove.record({ application: '48211' }, {
//     stream: 'loan-decisions', actor: 'underwriting-model-v3', action: 'loan.decision',
//   })
//
// The record's content never leaves your process: only its SHA-256 is sent.

import { createHash } from 'node:crypto'

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

export class Prove {
  constructor(baseUrl, apiKey, { timeoutMs = 15000 } = {}) {
    this.baseUrl = baseUrl.replace(/\/+$/, '')
    this.apiKey = apiKey
    this.timeoutMs = timeoutMs
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
  record(record, opts) {
    return this.#request('POST', '/v1/ingest', this.#item(record, opts))
  }

  /** Record up to 100 items in order: [{ record, actor, action, ... }]. */
  async recordBatch(items, { stream } = {}) {
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
