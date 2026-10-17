import { useState } from 'react'
import { Link } from 'react-router-dom'
import { useOrg } from '../auth.jsx'
import { ago, fmtDate } from '../format.js'
import { Badge, Card, CopyField, Empty, Gate, Modal, PageHeader, useLoad, useToast } from '../ui.jsx'

export default function ApiKeys() {
  const org = useOrg()
  const toast = useToast()
  const st = useLoad(async () => {
    if (!org.isAdmin) return { keys: [], streams: [], denied: true }
    const [k, s] = await Promise.all([org.get('/api-keys'), org.get('/streams')])
    return { keys: k.keys, streams: s.streams.filter((x) => x.kind === 'user' && !x.archived) }
  }, [org.id])
  const [creating, setCreating] = useState(false)
  const [f, setF] = useState({ name: '', scope: 'ingest', stream_id: '' })
  const [err, setErr] = useState('')
  const [fresh, setFresh] = useState(null)

  const create = async (e) => {
    e.preventDefault()
    setErr('')
    try {
      setFresh(await org.post('/api-keys', { ...f, stream_id: f.stream_id || null }))
      setCreating(false)
      setF({ name: '', scope: 'ingest', stream_id: '' })
      st.reload()
    } catch (x) { setErr(x.message) }
  }
  const revoke = async (k) => {
    if (!window.confirm(`Revoke “${k.name}”? Systems using it will immediately get 401 errors.`)) return
    try { await org.del(`/api-keys/${k.id}`); toast('Key revoked'); st.reload() } catch (x) { toast(x.message, 'bad') }
  }

  return (
    <>
      <PageHeader title="API keys" sub="Machine credentials for recording and reading proofs.">
        {org.isAdmin && <button className="btn primary" onClick={() => setCreating(true)}>Create key</button>}
      </PageHeader>
      <Gate state={st}>
        {(d) => d.denied ? (
          <Card><Empty title="Admins only">Ask an organization admin or owner to manage API keys.</Empty></Card>
        ) : d.keys.length === 0 ? (
          <Card><Empty title="No API keys yet">Create an <strong>ingest</strong> key, bind it to a stream, and follow the <Link to="/quickstart">quickstart</Link>.</Empty></Card>
        ) : (
          <Card flush>
            <div className="table-wrap">
              <table className="t">
                <thead><tr><th>Name</th><th>Key</th><th>Scope</th><th>Stream</th><th>Last used</th><th /></tr></thead>
                <tbody>
                  {d.keys.map((k) => (
                    <tr key={k.id}>
                      <td><strong>{k.name}</strong><div className="small muted">created {fmtDate(k.created_at)}</div></td>
                      <td><code>{k.key_prefix}…</code></td>
                      <td><Badge kind={k.scope === 'ingest' ? 'brand' : ''}>{k.scope}</Badge></td>
                      <td>{k.stream_name || <span className="muted">any</span>}</td>
                      <td>{ago(k.last_used_at)}</td>
                      <td style={{ textAlign: 'right' }}>{k.revoked_at ? <Badge kind="bad">Revoked</Badge> : <button className="btn sm danger" onClick={() => revoke(k)}>Revoke</button>}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
        )}
      </Gate>
      {creating && (
          <Modal title="Create API key" onClose={() => setCreating(false)} footer={<><button className="btn" onClick={() => setCreating(false)}>Cancel</button><button className="btn primary" form="new-key">Create</button></>}>
            <form id="new-key" className="col" onSubmit={create}>
              <label className="f">Name<input required autoFocus value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} placeholder="underwriting-service (production)" /></label>
              <label className="f">Scope
                <select value={f.scope} onChange={(e) => setF({ ...f, scope: e.target.value })}>
                  <option value="ingest">ingest: record proofs</option>
                  <option value="read">read: fetch proofs and bundles</option>
                </select>
              </label>
              <label className="f">Restrict to a stream <span className="hint">recommended: limits damage if the key leaks</span>
                <select value={f.stream_id} onChange={(e) => setF({ ...f, stream_id: e.target.value })}>
                  <option value="">Any stream</option>
                  {(st.data?.streams || []).map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}
                </select>
              </label>
              {err && <div className="err">{err}</div>}
            </form>
          </Modal>
      )}
      {fresh && (
        <Modal title="Copy your API key" onClose={() => setFresh(null)} footer={<button className="btn primary" onClick={() => setFresh(null)}>I have saved it</button>}>
          <div className="col">
            <div className="note warn">This is the only time the full key is shown. We store just a hash, so we cannot recover it.</div>
            <CopyField value={fresh.token} />
            <p className="small muted">Keep it in your secrets manager or environment, never in source control.</p>
          </div>
        </Modal>
      )}
    </>
  )
}
