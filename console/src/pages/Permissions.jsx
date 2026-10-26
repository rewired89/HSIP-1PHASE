import { useState } from 'react'
import { useOrg } from '../auth.jsx'
import { fmtDate, fmtNum, short } from '../format.js'
import { Badge, Card, Empty, Gate, Modal, PageHeader, useLoad, useToast } from '../ui.jsx'

function describe(m) {
  const r = m.rules || {}
  const parts = []
  if (r.allowed_actions?.length) parts.push(`actions: ${r.allowed_actions.join(', ')}`)
  if (r.max_amount) parts.push(`${r.max_amount.field} up to ${fmtNum(r.max_amount.limit)}`)
  if (r.max_per_period) parts.push(`at most ${r.max_per_period.count} per ${r.max_per_period.period_secs}s`)
  for (const [k, v] of Object.entries(r.allowed_values || {})) parts.push(`${k}: ${v.join(', ')}`)
  return parts.length ? parts.join(' · ') : 'any action'
}

function AddPermission({ onClose, onDone }) {
  const org = useOrg()
  const toast = useToast()
  const [text, setText] = useState('')
  const [err, setErr] = useState('')
  const [busy, setBusy] = useState(false)
  const submit = async (e) => {
    e.preventDefault()
    setErr('')
    let body
    try { body = JSON.parse(text) } catch { setErr('That is not valid JSON.'); return }
    setBusy(true)
    try { await org.post('/mandates', body); toast('Permission added'); onDone(); onClose() } catch (x) { setErr(x.message) } finally { setBusy(false) }
  }
  return (
    <Modal title="Add a permission" onClose={onClose} footer={<><button className="btn" onClick={onClose}>Cancel</button><button className="btn primary" form="add-perm" disabled={busy || !text.trim()}>{busy ? 'Checking…' : 'Add permission'}</button></>}>
      <form id="add-perm" className="col" onSubmit={submit}>
        <p className="muted" style={{ margin: 0 }}>A permission is signed on your own machine, so HSIP never holds the key that grants it. The easiest way is the “Create a permission” form on the Proof Check page (`verifier/`), which signs in your browser. Or from the command line, write the rules in a file, then sign it:</p>
        <pre className="code" style={{ margin: 0 }}>{`hsip-proof keygen > my-key.json
hsip-proof mandate-sign my-key.json rules.json > permission.json`}</pre>
        <label className="f">Paste permission.json<textarea rows={9} spellCheck={false} value={text} onChange={(e) => setText(e.target.value)} style={{ fontFamily: 'monospace' }} /></label>
        {err && <div className="err">{err}</div>}
      </form>
    </Modal>
  )
}

function Report({ mandate, streams, onClose }) {
  const org = useOrg()
  const [sid, setSid] = useState(streams.find((s) => s.kind === 'user')?.id || streams[0]?.id || '')
  const [res, setRes] = useState(null)
  const [err, setErr] = useState('')
  const [busy, setBusy] = useState(false)
  const run = async () => {
    setBusy(true); setErr(''); setRes(null)
    try { setRes(await org.get(`/mandates/${encodeURIComponent(mandate.mandate_id)}/report?stream_id=${encodeURIComponent(sid)}`)) } catch (x) { setErr(x.message) } finally { setBusy(false) }
  }
  const r = res?.report
  return (
    <Modal title={`Check records: ${mandate.agent}`} onClose={onClose} wide footer={<button className="btn" onClick={onClose}>Close</button>}>
      <div className="col">
        <div className="row wrap">
          <select value={sid} onChange={(e) => setSid(e.target.value)} aria-label="Stream">
            {streams.map((s) => <option key={s.id} value={s.id}>{s.name} · {fmtNum(s.proof_count)} records</option>)}
          </select>
          <button className="btn primary" onClick={run} disabled={busy || !sid}>{busy ? 'Checking…' : 'Check this stream'}</button>
        </div>
        {err && <div className="err">{err}</div>}
        {r && (r.checked === 0
          ? <div className="note">No records by “{mandate.agent}” in this stream, so nothing was checked.</div>
          : <>
            <div className="note"><strong>{r.within_permission ? 'All records are inside the permission.' : `${r.violations.length} outside the permission.`}</strong> {r.checked} records checked for this agent, {r.ignored} by other actors ignored. A record can be genuine and still outside what was allowed.</div>
            {r.violations.length > 0 && (
              <div className="table-wrap"><table className="t">
                <thead><tr><th>Record</th><th>Rule</th><th>Why</th></tr></thead>
                <tbody>{r.violations.map((v, i) => <tr key={i}><td>#{v.seq}<div className="hash small">{short(v.event_hash, 8, 4)}</div></td><td><Badge kind="bad">{v.rule}</Badge></td><td className="small">{v.detail}</td></tr>)}</tbody>
              </table></div>
            )}
            <p className="small muted" style={{ margin: 0 }}>Anyone can repeat this offline: export an evidence pack, then <code>hsip-proof mandate-check permission.json pack.json</code>, or use the Proof Check page.</p>
          </>)}
      </div>
    </Modal>
  )
}

export default function Permissions() {
  const org = useOrg()
  const toast = useToast()
  const st = useLoad(async () => {
    const [m, s] = await Promise.all([org.get('/mandates'), org.get('/streams')])
    return { mandates: m.mandates, streams: s.streams }
  }, [org.id])
  const [adding, setAdding] = useState(false)
  const [checking, setChecking] = useState(null)
  const revoke = async (m) => {
    if (!window.confirm(`Revoke the permission for ${m.agent}? Records made after this moment will show as outside it on this platform.`)) return
    try { await org.post(`/mandates/${encodeURIComponent(m.mandate_id)}/revoke`, {}); toast('Revoked'); st.reload() } catch (x) { toast(x.message, 'bad') }
  }
  return (
    <>
      <PageHeader title="Permissions" sub="Say what an agent may do, then see which recorded actions fall outside it.">
        {org.canWrite && <button className="btn primary" onClick={() => setAdding(true)}>Add permission</button>}
      </PageHeader>
      <Gate state={st}>
        {({ mandates, streams }) => (
          <div className="col" style={{ gap: 16 }}>
            <div className="note">This proves after the fact what was recorded against what you allowed. It does not stop an agent, it cannot see actions that were never recorded, and the numbers it checks (such as an amount) are declared by the system that records them.</div>
            {mandates.length === 0 ? <Card><Empty title="No permissions yet">Add a signed permission to start checking an agent's recorded actions against it.</Empty></Card> : (
              <Card flush>
                <div className="table-wrap"><table className="t">
                  <thead><tr><th>Agent</th><th>Rules</th><th>Granted by</th><th>Status</th><th /></tr></thead>
                  <tbody>{mandates.map((m) => (
                    <tr key={m.mandate_id}>
                      <td><strong>{m.agent}</strong><div className="small muted">{m.mandate_id}</div></td>
                      <td className="small">{describe(m.mandate)}</td>
                      <td><span className="hash">{short(m.grantor_key, 8, 4)}</span><div className="small muted">{fmtDate(m.created_at)}</div></td>
                      <td>{m.revoked_at ? <Badge kind="bad">Revoked</Badge> : <Badge kind="ok">Active</Badge>}</td>
                      <td><div className="row" style={{ justifyContent: 'flex-end' }}>
                        <button className="btn sm" onClick={() => setChecking(m)}>Check records</button>
                        {org.isAdmin && !m.revoked_at && <button className="btn sm" onClick={() => revoke(m)}>Revoke</button>}
                      </div></td>
                    </tr>
                  ))}</tbody>
                </table></div>
              </Card>
            )}
          </div>
        )}
      </Gate>
      {adding && <AddPermission onClose={() => setAdding(false)} onDone={st.reload} />}
      {checking && st.data && <Report mandate={checking} streams={st.data.streams} onClose={() => setChecking(null)} />}
    </>
  )
}
