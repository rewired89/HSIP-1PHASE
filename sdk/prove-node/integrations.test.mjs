import test from 'node:test'
import assert from 'node:assert/strict'
import http from 'node:http'
import { HsipSpanProcessor, spanContent } from './otel.mjs'
import { makeHandler } from './webhook-relay.mjs'
import { hashPayload } from './index.mjs'

const span = (attributes) => ({
  name: 'loan.decision', attributes, startTime: [1, 5], endTime: [2, 6], status: { code: 1 },
  spanContext: () => ({ traceId: 'a'.repeat(32), spanId: 'b'.repeat(16) }),
})

test('otel: records only selected spans, sends hash and ids', async () => {
  const calls = []
  const prove = { record: async (r, o) => { calls.push([r, o]); return { proof_id: 'p' } } }
  const p = new HsipSpanProcessor(prove, { stream: 's' })
  const sel = span({ 'hsip.record': true, 'hsip.subject': 'app-1' })
  p.onEnd(sel); p.onEnd(span({ x: 1 }))
  await p.shutdown()
  assert.equal(calls.length, 1)
  assert.equal(calls[0][1].payloadHash, hashPayload(spanContent(sel)))
  assert.equal(calls[0][1].subject, 'app-1')
  assert.deepEqual(calls[0][1].metadata, { trace_id: 'a'.repeat(32), span_id: 'b'.repeat(16) })
})

test('otel: failures are swallowed', async () => {
  let err
  const p = new HsipSpanProcessor({ record: async () => { throw new Error('down') } }, { onError: (e) => { err = e } })
  p.onEnd(span({ 'hsip.record': true }))
  await p.shutdown()
  assert.equal(err.message, 'down')
})

test('relay: auth required, body is hashed not forwarded', async () => {
  const seen = []
  const prove = { record: async (body, o) => { seen.push([body, o]); return { proof_id: 'p9' } } }
  const srv = http.createServer(makeHandler(prove, { token: 'secret', stream: 'z' }))
  await new Promise((r) => srv.listen(0, r))
  const base = `http://127.0.0.1:${srv.address().port}`
  let r = await fetch(`${base}/record`, { method: 'POST', body: '{}' })
  assert.equal(r.status, 401)
  r = await fetch(`${base}/record?action=a.b&subject=S`, { method: 'POST', headers: { authorization: 'Bearer secret' }, body: '{"k":1}' })
  assert.equal(r.status, 200)
  assert.equal((await r.json()).proof_id, 'p9')
  assert.equal(seen[0][0].toString(), '{"k":1}')
  assert.equal(seen[0][1].action, 'a.b')
  srv.close()
})
