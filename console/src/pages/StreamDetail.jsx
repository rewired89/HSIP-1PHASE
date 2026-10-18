import { useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { useOrg } from '../auth.jsx'
import { ago, fmtNum, sha256Hex } from '../format.js'
import { Badge, Card, Gate, Modal, PageHeader, Stat, useLoad, useToast } from '../ui.jsx'
import { RecordsTable } from './Records.jsx'

function TestProof({ stream, onClose, onDone }) {
  const org = useOrg()
  const toast = useToast()
  const [f, setF] = useState({ action: 'test.event', subject: '', content: '' })
  const [err, setErr] = useState('')
  const [busy, setBusy] = useState(false)
  const submit = async (e) => {
    e.preventDefault()
    setBusy(true)
    setErr('')
    try {
      const payload_hash = await sha256Hex(f.content)
      await org.post(`/streams/${stream.id}/proofs`, { action: f.action, subject: f.subject, payload_hash })
      toast('Proof recorded')
      onDone()
      onClose()
    } catch (x) { setErr(x.message) } finally { setBusy(false) }
  }
  return (
    <Modal title="Send a test proof" onClose={onClose} footer={<><button className="btn" onClick={onClose}>Cancel</button><button className="btn primary" form="test-proof" disabled={busy || !f.content}>Record proof</button></>}>
      <form id="test-proof" className="col" onSubmit={submit}>
        <div className="note">The text below is hashed in your browser. Only the SHA-256 fingerprint is sent, exactly as your systems would do it.</div>
        <label className="f">Action<input required value={f.action} onChange={(e) => setF({ ...f, action: e.target.value })} pattern="[A-Za-z0-9._:\/\-]+" /><span className="hint">Letters, digits and . _ : / -</span></label>
        <label className="f">Subject <span className="hint">optional, e.g. an application or order id</span><input value={f.subject} onChange={(e) => setF({ ...f, subject: e.target.value })} /></label>
        <label className="f">Record content<textarea required value={f.content} onChange={(e) => setF({ ...f, content: e.target.value })} placeholder='{"decision":"approve","score":0.91}' /></label>
        {err && <div className="err">{err}</div>}
      </form>
    </Modal>
  )
}

export default function StreamDetail() {
  const { sid } = useParams()
  const org = useOrg()
  const toast = useToast()
  const st = useLoad(async () => {
    const d = await org.get('/streams')
    const s = d.streams.find((x) => x.id === sid)
    if (!s) throw new Error('Stream not found')
    return s
  }, [org.id, sid])
  const own = useLoad(() => org.get(''), [org.id])
  const customer = own.data?.custody?.mode === 'customer'
  const [testing, setTesting] = useState(false)
  const [tick, setTick] = useState(0)
  const [integrity, setIntegrity] = useState(null)
  const [checking, setChecking] = useState(false)

  const check = async () => {
    setChecking(true)
    try { setIntegrity(await org.get(`/streams/${sid}/integrity`)) } catch (e) { toast(e.message, 'bad') } finally { setChecking(false) }
  }
  const archive = async (archived) => {
    try { await org.patch(`/streams/${sid}`, { archived }); toast(archived ? 'Stream archived' : 'Stream restored'); st.reload() } catch (e) { toast(e.message, 'bad') }
  }

  return (
    <Gate state={st}>
      {(s) => (
        <>
          <PageHeader title={s.name} sub={s.description || `Stream · ${s.slug}`}>
            {s.archived && <Badge>Archived</Badge>}
            <Link className="btn" to="/evidence" state={{ stream: s.id }}>Export evidence</Link>
            <button className="btn" onClick={check} disabled={checking}>{checking ? 'Checking…' : 'Check integrity'}</button>
            {org.canWrite && !s.archived && !customer && <button className="btn primary" onClick={() => setTesting(true)}>Send test proof</button>}
            {customer && <span className="small muted">Your own key signs here, so the console can't send test proofs. Use an SDK.</span>}
          </PageHeader>
          <div className="col" style={{ gap: 16 }}>
            <div className="grid g4">
              <Stat label="Records" value={fmtNum(s.proof_count)} />
              <Stat label="Anchored" value={s.proof_count ? `${Math.round((s.anchored_count / s.proof_count) * 100)}%` : '–'} detail={`${fmtNum(s.anchored_count)} of ${fmtNum(s.proof_count)}`} />
              <Stat label="Last record" value={ago(s.last_proof_at)} />
              <Stat label="Stream ID" value={<span className="mono small">{s.id.slice(0, 13)}…</span>} detail={`slug: ${s.slug}`} />
            </div>
            {integrity && (
              <div className={`verdict ${integrity.intact ? 'ok' : 'bad'}`} role="status">
                <div style={{ fontSize: 28 }}>{integrity.intact ? '✓' : '✕'}</div>
                <div>
                  <div className="big">{integrity.intact ? 'Chain intact' : `Chain broken at record #${integrity.first_break_seq}`}</div>
                  <div className="small">
                    {integrity.intact
                      ? `${fmtNum(integrity.checked)} records re-verified from the database: hashes, signatures, links and batch proofs all match.`
                      : `${integrity.reason}. ${fmtNum(integrity.checked)} records checked before the break.`}
                  </div>
                </div>
              </div>
            )}
            <RecordsTable streamId={sid} reloadKey={tick} />
            {org.isAdmin && (
              <div className="row">
                <span className="grow" />
                {s.archived
                  ? <button className="btn sm" onClick={() => archive(false)}>Restore stream</button>
                  : <button className="btn sm danger" onClick={() => window.confirm('Archive this stream? It stops accepting new records; existing records stay verifiable.') && archive(true)}>Archive stream</button>}
              </div>
            )}
          </div>
          {testing && <TestProof stream={s} onClose={() => setTesting(false)} onDone={() => { setTick((t) => t + 1); st.reload() }} />}
        </>
      )}
    </Gate>
  )
}
