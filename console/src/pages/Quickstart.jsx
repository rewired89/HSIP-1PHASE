import { useState } from 'react'
import { Link } from 'react-router-dom'
import { useOrg } from '../auth.jsx'
import { Card, CodeBlock, Gate, PageHeader, Tabs, useLoad } from '../ui.jsx'

export default function Quickstart() {
  const org = useOrg()
  const [tab, setTab] = useState('curl')
  const st = useLoad(() => org.get('/streams'), [org.id])
  const base = window.location.origin

  return (
    <>
      <PageHeader title="Quickstart" sub="Record your first proof in under five minutes." />
      <Gate state={st}>
        {(d) => {
          const stream = d.streams.find((s) => s.kind === 'user')
          const slug = stream?.slug || 'loan-decisions'
          const curl = `# 1. Hash the record on your side. The content never leaves your systems.
PAYLOAD='{"application":"48211","decision":"approve","score":0.91}'
HASH=$(printf '%s' "$PAYLOAD" | sha256sum | cut -d' ' -f1)

# 2. Record the proof.
curl -sS ${base}/v1/ingest \\
  -H "Authorization: Bearer $HSIP_PROVE_KEY" \\
  -H "Content-Type: application/json" \\
  -d "{
    \\"stream\\": \\"${slug}\\",
    \\"actor\\": \\"underwriting-model-v3\\",
    \\"action\\": \\"loan.decision\\",
    \\"subject\\": \\"application-48211\\",
    \\"payload_hash\\": \\"$HASH\\",
    \\"metadata\\": {\\"region\\": \\"eu\\"}
  }"`
          const python = `import hashlib, json, os, requests

BASE = "${base}"
KEY = os.environ["HSIP_PROVE_KEY"]

def prove(record: dict, actor: str, action: str, subject: str = "", stream: str = "${slug}"):
    # Hash locally: only the fingerprint is sent.
    canonical = json.dumps(record, sort_keys=True, separators=(",", ":")).encode()
    r = requests.post(
        f"{BASE}/v1/ingest",
        headers={"Authorization": f"Bearer {KEY}"},
        json={
            "stream": stream, "actor": actor, "action": action, "subject": subject,
            "payload_hash": hashlib.sha256(canonical).hexdigest(),
        },
        timeout=10,
    )
    r.raise_for_status()
    return r.json()["proof_id"]

proof_id = prove({"application": "48211", "decision": "approve"},
                 actor="underwriting-model-v3", action="loan.decision", subject="application-48211")
print("proof", proof_id)`
          const node = `import { createHash } from 'node:crypto'

const BASE = '${base}'
const KEY = process.env.HSIP_PROVE_KEY

export async function prove(record, { actor, action, subject = '', stream = '${slug}' }) {
  // Hash locally: only the fingerprint is sent.
  const payload_hash = createHash('sha256').update(JSON.stringify(record)).digest('hex')
  const res = await fetch(\`\${BASE}/v1/ingest\`, {
    method: 'POST',
    headers: { Authorization: \`Bearer \${KEY}\`, 'Content-Type': 'application/json' },
    body: JSON.stringify({ stream, actor, action, subject, payload_hash }),
  })
  if (!res.ok) throw new Error((await res.json()).error?.message ?? res.statusText)
  return (await res.json()).proof_id
}`
          const own = `# Customer-held key: records are signed on your machine, the platform never sees the key.
# 1. hsip-proof keygen > hsip-key.json      (keep it secret, back it up)
# 2. register it once in Settings > Key custody

# Python (pip install cryptography)
from hsip_prove import Prove
prove = Prove("${base}", os.environ["HSIP_PROVE_KEY"], signing_key_b64=open("hsip-key.json").read())
prove.record({"application": "48211"}, actor="underwriting-model-v3", action="loan.decision", stream="${slug}")

// Node 18+
import { Prove } from './index.mjs'
const prove = new Prove('${base}', process.env.HSIP_PROVE_KEY, { signingKey: fs.readFileSync('hsip-key.json', 'utf8') })
await prove.record({ application: '48211' }, { actor: 'underwriting-model-v3', action: 'loan.decision', stream: '${slug}' })

# Without an SDK: POST /v1/ingest/prepare (same body as /v1/ingest) returns {envelope, event_hash};
# check event_hash against the envelope, sign its 32 bytes with Ed25519, then
# POST /v1/ingest/commit {envelope, signature}.`
          const code = { curl, python, node, own }[tab]
          return (
            <div className="col" style={{ gap: 16 }}>
              <Card title="1. Create a key">
                <p>Create an <strong>ingest</strong> key bound to a stream on the <Link to="/api-keys">API keys</Link> page, then export it:</p>
                <CodeBlock code="export HSIP_PROVE_KEY=hsp_…" />
                {!stream && <p className="small muted" style={{ marginTop: 10 }}>You have no streams yet. Create one on the <Link to="/streams">Streams</Link> page; the examples below use the slug “{slug}”.</p>}
              </Card>
              <Card title="2. Record a proof">
                <Tabs tabs={[['curl', 'curl'], ['python', 'Python'], ['node', 'Node.js'], ['own', 'Your own key']]} value={tab} onChange={setTab} />
                <CodeBlock code={code} />
                <p className="small muted" style={{ marginTop: 10 }}>Batch up to 100 records per call with <code>POST /v1/ingest/batch</code> and <code>{'{"items": [...]}'}</code>. Records in one stream are chained in the order they are accepted, so use one stream per independent system to keep write throughput high.</p>
              </Card>
              <Card title="3. Verify it independently">
                <p>Fetch the bundle with a <code>read</code> key (<code>GET /v1/proofs/&lt;id&gt;</code>) or download it from the console, then verify with the open-source tool. No account, no network:</p>
                <CodeBlock code={`hsip-proof verify proof-<id>.json\n# RESULT: VALID`} />
                <p className="small muted" style={{ marginTop: 10 }}>Build it from source: <code>cargo install --path crates/hsip-proof</code>. The verifier is the same code the platform uses, and the format is documented in <code>docs/PROVE_PROOF_FORMAT.md</code>.</p>
              </Card>
              <Card title="What to hash">
                <p className="muted" style={{ margin: 0 }}>Hash the exact bytes you could later produce again: the decision input and output, the model version, and anything an auditor would ask about. Keep the original record in your own storage. Include a random salt inside what you hash: a guessable record such as “approve” can otherwise be recovered from its hash. To prove a record later, hash the original and compare it with the proof's payload hash; the “Check original” tab does this for you.</p>
              </Card>
            </div>
          )
        }}
      </Gate>
    </>
  )
}
