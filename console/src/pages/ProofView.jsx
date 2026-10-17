import { useState } from 'react'
import { api } from '../api.js'
import { download, fmtDate, short } from '../format.js'
import { Badge, Card, Checks, CodeBlock, Tabs, Verdict } from '../ui.jsx'

export function Fact({ k, children }) {
  return (<><dt>{k}</dt><dd>{children}</dd></>)
}

/** Original-content check: hashes locally on the server only for comparison. */
export function ContentMatch({ bundle }) {
  const [text, setText] = useState('')
  const [res, setRes] = useState(null)
  const [busy, setBusy] = useState(false)
  const run = async () => {
    setBusy(true)
    try { setRes((await api.post('/v1/verify/match', { bundle, content: text }, { token: null })).matches) } catch { setRes(null) } finally { setBusy(false) }
  }
  return (
    <div className="col">
      <p className="muted">We store only a fingerprint of the original record. Paste the original here to confirm it is the exact record this proof covers (byte-for-byte, including whitespace). It is hashed to compare and is not stored.</p>
      <textarea value={text} onChange={(e) => { setText(e.target.value); setRes(null) }} placeholder="Paste the original record…" />
      <div className="row">
        <button className="btn primary" onClick={run} disabled={busy || !text}>Compare</button>
        {res === true && <Badge kind="ok">Match: this is the recorded content</Badge>}
        {res === false && <Badge kind="bad">No match: different content</Badge>}
      </div>
    </div>
  )
}

export default function ProofView({ bundle, verification, extra, actions }) {
  const [tab, setTab] = useState('summary')
  const e = bundle.envelope
  const a = bundle.anchor
  return (
    <div className="col" style={{ gap: 16 }}>
      <Verdict valid={verification.valid} anchored={verification.anchored} />
      <Card>
        <Tabs tabs={[['summary', 'Summary'], ['checks', 'Verification'], ['content', 'Check original'], ['raw', 'Proof bundle']]} value={tab} onChange={setTab} />
        {tab === 'summary' && (
          <dl className="kv">
            <Fact k="Organization">{bundle.org_name}</Fact>
            <Fact k="Action"><code>{e.action}</code></Fact>
            <Fact k="Actor">{e.actor}</Fact>
            <Fact k="Subject">{e.subject || '–'}</Fact>
            <Fact k="Recorded">{fmtDate(e.timestamp_ms)} <span className="muted">({e.timestamp_iso})</span></Fact>
            <Fact k="Sequence">#{e.seq} in its stream</Fact>
            {Object.entries(e.metadata || {}).map(([k, v]) => <Fact key={k} k={`meta · ${k}`}>{v}</Fact>)}
            <Fact k="Payload hash"><span className="hash">{e.payload_hash}</span></Fact>
            <Fact k="Record hash"><span className="hash">{bundle.event_hash}</span></Fact>
            <Fact k="Previous hash">{e.prev_hash ? <span className="hash">{e.prev_hash}</span> : <span className="muted">none (first record)</span>}</Fact>
            <Fact k="Batch">
              {a ? (
                <>
                  Record {a.merkle_index + 1} of {a.leaf_count} · root <span className="hash">{short(a.merkle_root, 10, 6)}</span><br />
                  <span className="small muted">Anchored {fmtDate(Date.parse(a.anchored_at_iso))} · Bitcoin timestamp: {a.ots_status === 'confirmed' ? 'confirmed' : a.ots_status === 'pending' ? 'pending confirmation' : a.ots_status === 'calendar_unreachable' ? 'retrying' : 'not enabled'}</span>
                </>
              ) : <span className="muted">Not anchored yet. This happens automatically within about a minute.</span>}
            </Fact>
            {extra}
          </dl>
        )}
        {tab === 'checks' && <Checks checks={verification.checks} />}
        {tab === 'content' && <ContentMatch bundle={bundle} />}
        {tab === 'raw' && (
          <div className="col">
            <div className="row">
              <span className="muted small grow">Self-contained. Verify it anywhere with <code>hsip-proof verify proof.json</code>.</span>
              <button className="btn sm" onClick={() => download(`proof-${e.proof_id}.json`, JSON.stringify(bundle, null, 2))}>Download</button>
            </div>
            <CodeBlock code={JSON.stringify(bundle, null, 2)} />
          </div>
        )}
      </Card>
      {actions}
    </div>
  )
}
