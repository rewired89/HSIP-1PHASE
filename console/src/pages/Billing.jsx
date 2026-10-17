import { useState } from 'react'
import { useAuth, useOrg } from '../auth.jsx'
import { fmtNum, money } from '../format.js'
import { api } from '../api.js'
import { Badge, Card, Gate, Meter, Modal, PageHeader, useLoad, useToast } from '../ui.jsx'

export default function Billing() {
  const org = useOrg()
  const { refresh } = useAuth()
  const toast = useToast()
  const st = useLoad(async () => {
    const [u, meta] = await Promise.all([org.get('/usage'), api.get('/v1/meta', { token: null })])
    return { u, plans: meta.plans }
  }, [org.id])
  const [req, setReq] = useState(null)
  const [msg, setMsg] = useState('')
  const [err, setErr] = useState('')

  const send = async (e) => {
    e.preventDefault()
    setErr('')
    try {
      await org.post('/plan-request', { plan: req.id, message: msg })
      toast('Request sent. We will confirm shortly.')
      setReq(null)
      setMsg('')
      refresh()
    } catch (x) { setErr(x.message) }
  }

  return (
    <>
      <PageHeader title="Plan & usage" sub="What you have used this period and what each plan includes." />
      <Gate state={st}>
        {({ u, plans }) => {
          const cur = u.plan
          const max = Math.max(1, ...u.history.map((h) => h.proofs))
          return (
            <div className="col" style={{ gap: 16 }}>
              <div className="grid g-main">
                <Card title={`${cur.name} plan`} actions={<Badge kind="brand">{u.month}</Badge>}>
                  <div className="col" style={{ gap: 18 }}>
                    <div><div className="small muted" style={{ marginBottom: 4 }}>Proofs this month</div><Meter used={u.proofs_used} limit={cur.proofs_per_month} /></div>
                    <div><div className="small muted" style={{ marginBottom: 4 }}>Streams</div><Meter used={u.streams_used} limit={cur.streams} /></div>
                    <div><div className="small muted" style={{ marginBottom: 4 }}>Seats (members + pending invites)</div><Meter used={u.seats_used} limit={cur.seats} /></div>
                    {u.proofs_used >= cur.proofs_per_month && <div className="err">You have reached this plan's monthly limit. New proofs are rejected until next month or an upgrade.</div>}
                  </div>
                </Card>
                <Card title="Last 6 months">
                  {u.history.length === 0 ? <div className="muted">No usage yet.</div> : (
                    <div className="col">
                      {u.history.map((h) => (
                        <div key={h.month} className="row small">
                          <span style={{ width: 62 }} className="muted">{h.month}</span>
                          <div className="bar grow"><i style={{ width: `${(h.proofs / max) * 100}%` }} /></div>
                          <strong style={{ width: 70, textAlign: 'right' }}>{fmtNum(h.proofs)}</strong>
                        </div>
                      ))}
                    </div>
                  )}
                </Card>
              </div>
              <div className="grid g4">
                {plans.map((p) => (
                  <div key={p.id} className={`card price ${p.id === cur.id ? 'hot' : ''}`}>
                    <h3>{p.name} {p.id === cur.id && <Badge kind="brand">Current</Badge>}</h3>
                    <div className="amt">{money(p.price_usd_month)}{p.price_usd_month ? <span className="small muted"> /mo</span> : null}</div>
                    <div className="small muted">{fmtNum(p.proofs_per_month)} proofs/mo<br />{fmtNum(p.streams)} streams · {fmtNum(p.seats)} seats<br />Exports up to {fmtNum(p.evidence_max_proofs)} records</div>
                    <ul>{p.highlights.map((h) => <li key={h}>{h}</li>)}</ul>
                    <span className="grow" />
                    {p.id !== cur.id && org.isAdmin && <button className="btn" onClick={() => setReq(p)}>{p.price_usd_month === null ? 'Contact sales' : 'Request this plan'}</button>}
                  </div>
                ))}
              </div>
              <div className="note">Plan changes are confirmed by our team, usually within one business day. Existing records always stay verifiable, whatever your plan.</div>
            </div>
          )
        }}
      </Gate>
      {req && (
        <Modal title={`Request ${req.name}`} onClose={() => setReq(null)} footer={<><button className="btn" onClick={() => setReq(null)}>Cancel</button><button className="btn primary" form="plan-req">Send request</button></>}>
          <form id="plan-req" className="col" onSubmit={send}>
            <label className="f">Anything we should know? <span className="hint">expected volume, compliance needs, timeline</span><textarea maxLength={1000} value={msg} onChange={(e) => setMsg(e.target.value)} /></label>
            {err && <div className="err">{err}</div>}
          </form>
        </Modal>
      )}
    </>
  )
}
