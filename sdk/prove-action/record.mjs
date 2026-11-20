// GitHub Action entry. Hashes a file (or the commit SHA) locally and records only the hash.
import { readFileSync, appendFileSync } from 'node:fs'
import { Prove } from '../prove-node/index.mjs'

const input = (n) => process.env[`INPUT_${n.toUpperCase().replace(/-/g, '_')}`] || ''
const file = input('file')
const content = file ? readFileSync(file) : process.env.GITHUB_SHA || ''
const prove = new Prove(input('url'), input('api-key'))
const res = await prove.record(content, {
  stream: input('stream') || undefined,
  actor: input('actor'),
  action: input('action'),
  subject: input('subject'),
  metadata: { repository: process.env.GITHUB_REPOSITORY || '', run: process.env.GITHUB_RUN_ID || '' },
})
console.log(`HSIP proof ${res.proof_id}`)
if (process.env.GITHUB_OUTPUT) appendFileSync(process.env.GITHUB_OUTPUT, `proof-id=${res.proof_id}\n`)
