import { useRef, useState } from 'react'
import { api } from '../api.js'
import { Card, Checks, PageHeader, Verdict } from '../ui.jsx'
import ProofView from './ProofView.jsx'

export default function Verify() {
  const [text, setText] = useState('')
  const [res, setRes] = useState(null)
  const [err, setErr] = useState('')
  const [busy, setBusy] = useState(false)
  const file = useRef(null)

  const run = async (raw = text) => {
    setErr('')
    setRes(null)
    let json
    try { json = JSON.parse(raw) } catch { setErr('That is not valid JSON. Paste a proof bundle or evidence pack exactly as downloaded.'); return }
    setBusy(true)
    try {
      const r = await api.post('/v1/verify', json, { token: null })
      const bundle = r.kind === 'proof' ? (json.bundle && !json.format ? json.bundle : json) : null
      setRes({ ...r, bundle })
    } catch (e) { setErr(e.message) } finally { setBusy(false) }
  }

  const onFile = async (e) => {
    const f = e.target.files?.[0]
    if (!f) return
    const t = await f.text()
    setText(t)
    run(t)
  }

  return (
    <>
      <PageHeader title="Verify a proof" sub="Check any proof bundle or evidence pack. No account needed, and nothing is stored." />
      <div className="col" style={{ gap: 16 }}>
        <Card>
          <div className="col">
            <textarea style={{ minHeight: 160, fontFamily: 'var(--mono)', fontSize: 12.5 }} value={text} onChange={(e) => setText(e.target.value)} placeholder='Paste a proof bundle or evidence pack (JSON)…' aria-label="Proof JSON" />
            <div className="row wrap">
              <button className="btn primary" disabled={busy || !text.trim()} onClick={() => run()}>{busy ? 'Verifying…' : 'Verify'}</button>
              <button className="btn" onClick={() => file.current?.click()}>Upload file…</button>
              <input ref={file} type="file" accept="application/json,.json" hidden onChange={onFile} />
              <span className="muted small">For maximum assurance, run the open-source verifier offline: <code>hsip-proof verify file.json</code></span>
            </div>
            {err && <div className="err">{err}</div>}
          </div>
        </Card>
        {res && res.kind === 'proof' && res.bundle && <ProofView bundle={res.bundle} verification={res.result} />}
        {res && res.kind === 'evidence_pack' && (
          <div className="col" style={{ gap: 16 }}>
            <Verdict valid={res.valid} kind="pack" />
            <Card title={`${res.result.proof_count} records · ${res.result.anchored_count} anchored`}>
              <Checks checks={res.result.checks} />
              {res.result.invalid_proofs.length > 0 && <div className="err" style={{ marginTop: 12 }}>Failed records: {res.result.invalid_proofs.join(', ')}</div>}
            </Card>
          </div>
        )}
      </div>
    </>
  )
}
