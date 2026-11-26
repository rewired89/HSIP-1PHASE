// Smoke test for the compiled verifier that the Proof Check page loads
// (verifier/pkg). Run with: node --test crates/hsip-verify-web/smoke.test.mjs
import test from 'node:test'
import assert from 'node:assert/strict'
import { readFileSync } from 'node:fs'
import init, { verify, check_permission, public_key, sign_permission } from '../../verifier/pkg/hsip_verify_web.js'

const read = (rel) => readFileSync(new URL(rel, import.meta.url))
await init({ module_or_path: read('../../verifier/pkg/hsip_verify_web_bg.wasm') })

const proof = read('../../docs/examples/prove/sample-proof.json').toString('utf8')
const pack = read('../../docs/examples/prove/sample-evidence-pack.json').toString('utf8')
const seed = Buffer.alloc(32, 7).toString('base64')

const mandateBody = (rules, agent = 'underwriting-model-v3') =>
  JSON.stringify({
    format: 'hsip-mandate/1', mandate_id: 'm1', grantor_key: public_key(seed), agent,
    valid_from_ms: 0, valid_until_ms: 9e12, rules,
  })

test('the sample proof verifies and is anchored', () => {
  const r = JSON.parse(verify(proof))
  assert.equal(r.valid, true)
  assert.equal(r.result.anchored, true)
})

test('the sample evidence pack verifies', () => {
  assert.equal(JSON.parse(verify(pack)).valid, true)
})

test('an altered record fails', () => {
  const p = JSON.parse(proof)
  p.envelope.action = 'something.else'
  assert.equal(JSON.parse(verify(JSON.stringify(p))).valid, false)
})

test('a removed record from a pack fails', () => {
  const p = JSON.parse(pack)
  p.proofs.splice(1, 1)
  assert.equal(JSON.parse(verify(JSON.stringify(p))).valid, false)
})

test('garbage input returns an error, never throws', () => {
  for (const bad of ['', 'not json', '{}', '[]', '{"format":"hsip-evidence-pack/1"}']) {
    const r = JSON.parse(verify(bad))
    assert.ok(r.error || r.valid === false, bad)
  }
})

test('a misspelled permission rule is rejected, a correct one signs', () => {
  const typo = JSON.parse(sign_permission(seed, mandateBody({ allowed_action: ['loan.decision'] })))
  assert.ok(typo.error, 'typo must be rejected')
  const ok = JSON.parse(sign_permission(seed, mandateBody({ allowed_actions: ['loan.decision'] })))
  assert.ok(ok.mandate)
})

test('permission check passes an allowed action and flags a disallowed one', () => {
  const allowed = JSON.parse(sign_permission(seed, mandateBody({ allowed_actions: ['loan.decision'] }))).mandate
  const r1 = JSON.parse(check_permission(JSON.stringify(allowed), proof))
  assert.equal(r1.report.within_permission, true)
  const denied = JSON.parse(sign_permission(seed, mandateBody({ allowed_actions: ['other.action'] }))).mandate
  const r2 = JSON.parse(check_permission(JSON.stringify(denied), proof))
  assert.equal(r2.report.within_permission, false)
  assert.equal(r2.report.violations[0].rule, 'action_not_allowed')
})

test('time evidence is reported for a proof and summarized for a pack', () => {
  const r = JSON.parse(verify(proof))
  assert.equal(r.result.time_evidence.state, 'batch_only')
  const pk = JSON.parse(verify(pack))
  assert.deepEqual(Object.keys(pk.result.time_summary), ['batch_only'])
})

test('the pack with a removed record fails', () => {
  const removed = read('../../docs/examples/prove/sample-evidence-pack-removed-record.json').toString('utf8')
  const r = JSON.parse(verify(removed))
  assert.equal(r.valid, false)
})
