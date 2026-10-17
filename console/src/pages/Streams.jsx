import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { useOrg } from '../auth.jsx'
import { ago, fmtNum } from '../format.js'
import { Badge, Card, Empty, Gate, Modal, PageHeader, useLoad, useToast } from '../ui.jsx'

export default function Streams() {
  const org = useOrg()
  const nav = useNavigate()
  const toast = useToast()
  const st = useLoad(() => org.get('/streams'), [org.id])
  const [creating, setCreating] = useState(false)
  const [f, setF] = useState({ name: '', description: '' })
  const [err, setErr] = useState('')

  const create = async (e) => {
    e.preventDefault()
    setErr('')
    try {
      const r = await org.post('/streams', f)
      toast('Stream created')
      setCreating(false)
      setF({ name: '', description: '' })
      nav(`/streams/${r.id}`)
    } catch (x) { setErr(x.message) }
  }

  return (
    <>
      <PageHeader title="Streams" sub="A stream is one independent hash chain: usually one system, model or workflow.">
        {org.canWrite && <button className="btn primary" onClick={() => setCreating(true)}>New stream</button>}
      </PageHeader>
      <Gate state={st}>
        {(d) => {
          const streams = d.streams.filter((s) => s.kind === 'user')
          return streams.length === 0 ? (
            <Card><Empty title="No streams yet">Create one stream per system you want to prove, such as “Loan decisions” or “Trading agent”.</Empty></Card>
          ) : (
            <Card flush>
              <div className="table-wrap">
                <table className="t">
                  <thead><tr><th>Stream</th><th>Records</th><th>Anchored</th><th>Last record</th><th /></tr></thead>
                  <tbody>
                    {streams.map((s) => (
                      <tr key={s.id} className="click" onClick={() => nav(`/streams/${s.id}`)}>
                        <td><strong>{s.name}</strong><div className="small muted">{s.description || s.slug}</div></td>
                        <td>{fmtNum(s.proof_count)}</td>
                        <td>{s.proof_count ? `${Math.round((s.anchored_count / s.proof_count) * 100)}%` : '–'}</td>
                        <td>{ago(s.last_proof_at)}</td>
                        <td>{s.archived && <Badge>Archived</Badge>}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
          )
        }}
      </Gate>
      {creating && (
        <Modal title="New stream" onClose={() => setCreating(false)} footer={<><button className="btn" onClick={() => setCreating(false)}>Cancel</button><button className="btn primary" form="new-stream">Create</button></>}>
          <form id="new-stream" className="col" onSubmit={create}>
            <label className="f">Name<input required autoFocus maxLength={80} value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} placeholder="Loan decisions" /></label>
            <label className="f">Description <span className="hint">optional</span><input maxLength={300} value={f.description} onChange={(e) => setF({ ...f, description: e.target.value })} /></label>
            {err && <div className="err">{err}</div>}
          </form>
        </Modal>
      )}
    </>
  )
}
