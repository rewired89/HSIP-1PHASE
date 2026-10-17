import { useState } from 'react'
import { useLocation } from 'react-router-dom'
import { useOrg } from '../auth.jsx'
import { download, fmtDate, fmtDay, fmtNum, short, toInputDate } from '../format.js'
import { Badge, Card, Checks, Empty, Gate, Modal, PageHeader, useLoad, useToast } from '../ui.jsx'

function NewExport({ streams, initial, onClose, onDone }) {
  const org = useOrg()
  const toast = useToast()
  const [f, setF] = useState({
    stream_id: initial || streams.find((s) => s.kind === 'user')?.id || streams[0]?.id || '',
    from: toInputDate(Date.now() - 30 * 86400000),
    to: toInputDate(Date.now()),
    title: '',
  })
  const [err, setErr] = useState('')
  const [busy, setBusy] = useState(false)
  const [done, setDone] = useState(null)
  const submit = async (e) => {
    e.preventDefault()
    setBusy(true)
    setErr('')
    try {
      const r = await org.post('/evidence', {
        stream_id: f.stream_id,
        from_ms: new Date(f.from + 'T00:00:00').getTime(),
        to_ms: new Date(f.to + 'T23:59:59').getTime(),
        title: f.title,
      })
      setDone(r)
      onDone()
      toast('Evidence pack created')
    } catch (x) { setErr(x.message) } finally { setBusy(false) }
  }
  return (
    <Modal title="New evidence export" onClose={onClose} wide={!!done} footer={done ? <button className="btn primary" onClick={onClose}>Done</button> : <><button className="btn" onClick={onClose}>Cancel</button><button className="btn primary" form="new-export" disabled={busy}>{busy ? 'Building…' : 'Create export'}</button></>}>
      {done ? (
        <div className="col">
          <div className="note">{done.proof_count} records exported and re-verified before release ({done.anchored_count} anchored).</div>
          <Checks checks={done.verification.checks} />
        </div>
      ) : (
        <form id="new-export" className="col" onSubmit={submit}>
          <label className="f">Stream
            <select required value={f.stream_id} onChange={(e) => setF({ ...f, stream_id: e.target.value })}>
              {streams.map((s) => <option key={s.id} value={s.id}>{s.name}{s.kind === 'system' ? ' (audit trail)' : ''} · {fmtNum(s.proof_count)} records</option>)}
            </select>
          </label>
          <div className="grid g2">
            <label className="f">From<input type="date" required value={f.from} max={f.to} onChange={(e) => setF({ ...f, from: e.target.value })} /></label>
            <label className="f">To<input type="date" required value={f.to} min={f.from} onChange={(e) => setF({ ...f, to: e.target.value })} /></label>
          </div>
          <label className="f">Title <span className="hint">optional, appears on the report</span><input maxLength={120} value={f.title} onChange={(e) => setF({ ...f, title: e.target.value })} placeholder="Q4 2026 lending decisions" /></label>
          {err && <div className="err">{err}</div>}
        </form>
      )}
    </Modal>
  )
}

export default function Evidence() {
  const org = useOrg()
  const toast = useToast()
  const loc = useLocation()
  const st = useLoad(async () => {
    const [ex, streams] = await Promise.all([org.get('/evidence'), org.get('/streams')])
    return { exports: ex.exports, streams: streams.streams }
  }, [org.id])
  const [creating, setCreating] = useState(!!loc.state?.stream)

  const getPack = async (x) => {
    try {
      const res = await org.get(`/evidence/${x.id}/pack`, { raw: true })
      download(`evidence-${x.id}.json`, await res.text())
    } catch (e) { toast(e.message, 'bad') }
  }
  const openReport = async (x) => {
    try {
      const res = await org.get(`/evidence/${x.id}/report`, { raw: true })
      const html = await res.text()
      const url = URL.createObjectURL(new Blob([html], { type: 'text/html' }))
      window.open(url, '_blank', 'noopener')
      setTimeout(() => URL.revokeObjectURL(url), 60000)
    } catch (e) { toast(e.message, 'bad') }
  }

  return (
    <>
      <PageHeader title="Evidence" sub="Signed, complete exports of a stream over a period, for auditors, regulators and counterparties.">
        <button className="btn primary" onClick={() => setCreating(true)}>New export</button>
      </PageHeader>
      <Gate state={st}>
        {({ exports, streams }) => (
          <div className="col" style={{ gap: 16 }}>
            <div className="note">An evidence pack contains every record in the range with its signature and batch proof, plus a signed manifest listing exactly which records belong. Removing or adding a record breaks the manifest. Recipients verify it with <code>hsip-proof verify pack.json</code> or at <a href="/verify">/verify</a>.</div>
            {exports.length === 0 ? <Card><Empty title="No exports yet">Create your first pack to see what an auditor receives.</Empty></Card> : (
              <Card flush>
                <div className="table-wrap">
                  <table className="t">
                    <thead><tr><th>Export</th><th>Range</th><th>Records</th><th>Manifest</th><th>Created</th><th /></tr></thead>
                    <tbody>
                      {exports.map((x) => (
                        <tr key={x.id}>
                          <td><strong>{x.title}</strong><div className="small muted">{x.stream_name}</div></td>
                          <td className="small">{fmtDay(x.from_ms)} → {fmtDay(x.to_ms)}</td>
                          <td>{fmtNum(x.proof_count)}</td>
                          <td><span className="hash">{short(x.manifest_hash, 8, 4)}</span></td>
                          <td className="small">{fmtDate(x.created_at)}<div className="muted">{x.created_by}</div></td>
                          <td><div className="row" style={{ justifyContent: 'flex-end' }}><button className="btn sm" onClick={() => openReport(x)}>Report</button><button className="btn sm" onClick={() => getPack(x)}>Pack (JSON)</button></div></td>
                        </tr>
                      ))}
                    </tbody>
                  </table>
                </div>
              </Card>
            )}
            <Badge>Exports are immutable once created</Badge>
          </div>
        )}
      </Gate>
      {creating && st.data && (
        st.data.streams.filter((s) => s.proof_count > 0 || s.kind === 'user').length === 0
          ? <Modal title="Nothing to export" onClose={() => setCreating(false)}><p>Create a stream and record some proofs first.</p></Modal>
          : <NewExport streams={st.data.streams} initial={loc.state?.stream} onClose={() => setCreating(false)} onDone={st.reload} />
      )}
    </>
  )
}
