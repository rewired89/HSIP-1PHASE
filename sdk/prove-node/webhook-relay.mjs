// Webhook relay: receive JSON from Zapier, Make, n8n or any system that can POST a webhook,
// hash it here, and record only the hash in HSIP. Run it inside your own network so the
// content never reaches HSIP.
//
//   HSIP_URL=https://hsip.example.com HSIP_KEY=hsp_... HSIP_STREAM=zaps \
//   RELAY_TOKEN=choose-a-long-secret node webhook-relay.mjs      # listens on :8787
//
// POST /record?action=invoice.approved&actor=zapier&subject=INV-42
//   Authorization: Bearer <RELAY_TOKEN>      body: any JSON (hashed, not forwarded)

import http from 'node:http'
import { timingSafeEqual } from 'node:crypto'
import { Prove } from './index.mjs'

export function makeHandler(prove, { token, stream, maxBytes = 1_000_000 }) {
  const ok = (h) => {
    const a = Buffer.from(String(h || '')), b = Buffer.from(`Bearer ${token}`)
    return a.length === b.length && timingSafeEqual(a, b)
  }
  return async (req, res) => {
    const send = (code, obj) => { res.writeHead(code, { 'content-type': 'application/json' }); res.end(JSON.stringify(obj)) }
    const url = new URL(req.url, 'http://x')
    if (req.method !== 'POST' || url.pathname !== '/record') return send(404, { error: 'not found' })
    if (!ok(req.headers.authorization)) return send(401, { error: 'unauthorized' })
    const chunks = []
    let size = 0
    for await (const c of req) {
      size += c.length
      if (size > maxBytes) return send(413, { error: 'too large' })
      chunks.push(c)
    }
    try {
      const body = Buffer.concat(chunks)
      const r = await prove.record(body, {
        stream,
        actor: url.searchParams.get('actor') || 'webhook',
        action: url.searchParams.get('action') || 'webhook.event',
        subject: url.searchParams.get('subject') || '',
      })
      send(200, { proof_id: r.proof_id })
    } catch (e) {
      send(502, { error: 'could not record' })
    }
  }
}

if (import.meta.url === `file://${process.argv[1]}`) {
  const { HSIP_URL, HSIP_KEY, HSIP_STREAM, RELAY_TOKEN, PORT = '8787' } = process.env
  if (!HSIP_URL || !HSIP_KEY || !RELAY_TOKEN) throw new Error('set HSIP_URL, HSIP_KEY and RELAY_TOKEN')
  http.createServer(makeHandler(new Prove(HSIP_URL, HSIP_KEY), { token: RELAY_TOKEN, stream: HSIP_STREAM })).listen(+PORT)
}
