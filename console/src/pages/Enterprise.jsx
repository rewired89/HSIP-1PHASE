import { useState } from 'react'
import { Link } from 'react-router-dom'
import { api } from '../api.js'
import { useOrg } from '../auth.jsx'
import { Badge, Card, Gate, Modal, PageHeader, useLoad, useToast } from '../ui.jsx'

const STATUS = {
  available: ['ok', 'Available'],
  on_request: ['brand', 'On request'],
  roadmap: ['', 'Planned'],
}

export function ServiceGrid({ services, onRequest }) {
  return (
    <div className="grid g3">
      {services.map((s) => {
        const [kind, label] = STATUS[s.status] || ['', s.status]
        return (
          <Card key={s.id}>
            <div className="col" style={{ height: '100%' }}>
              <div className="row"><h3 className="grow">{s.name}</h3><Badge kind={kind}>{label}</Badge></div>
              <p className="muted" style={{ margin: 0 }}>{s.summary}</p>
              <span className="grow" />
              {onRequest && <button className="btn sm" style={{ alignSelf: 'flex-start' }} onClick={() => onRequest(s)}>{s.status === 'roadmap' ? 'Register interest' : 'Request this'}</button>}
            </div>
          </Card>
        )
      })}
    </div>
  )
}

export default function Enterprise() {
  const org = useOrg()
  const toast = useToast()
  const st = useLoad(() => api.get('/v1/meta', { token: null }), [])
  const [req, setReq] = useState(null)
  const [msg, setMsg] = useState('')
  const [err, setErr] = useState('')
  const hasOperator = (() => { try { return !!sessionStorage.getItem('hsip_operator') } catch { return false } })()

  const send = async (e) => {
    e.preventDefault()
    setErr('')
    try {
      await org.post('/service-request', { service: req.id, message: msg })
      toast('Request sent. Our team will reach out.')
      setReq(null)
      setMsg('')
    } catch (x) { setErr(x.message) }
  }

  return (
    <>
      <PageHeader title="Enterprise" sub="Everything we offer beyond the self-service plans. Pick what matters and we will follow up.">
        <Link className="btn" to="/billing">Compare plans</Link>
        {org.isAdmin && <Link className="btn primary" to="/billing">Request Enterprise</Link>}
      </PageHeader>
      <Gate state={st}>
        {(meta) => {
          const ent = meta.plans.find((p) => p.id === 'enterprise')
          return (
            <div className="col" style={{ gap: 16 }}>
              <Card title="Enterprise plan">
                <div className="grid g2">
                  <ul style={{ margin: 0, paddingLeft: 18 }}>{ent.highlights.map((h) => <li key={h}>{h}</li>)}</ul>
                  <div className="muted">
                    Custom volumes, dedicated or self-hosted deployment, and the services below. Pricing is agreed per deployment.
                    {!org.isAdmin && <div className="note" style={{ marginTop: 10 }}>Ask an organization admin or owner to send a request.</div>}
                  </div>
                </div>
              </Card>
              <ServiceGrid services={meta.services} onRequest={org.isAdmin ? setReq : null} />
              <div className="note">
                “Planned” items are not built yet. Registering interest helps us prioritize them, and we will not sell them as available.
                {hasOperator && <> You have the operator key loaded: <Link to="/operator">open the operator console</Link> to see requests.</>}
              </div>
            </div>
          )
        }}
      </Gate>
      {req && (
        <Modal title={req.name} onClose={() => setReq(null)} footer={<><button className="btn" onClick={() => setReq(null)}>Cancel</button><button className="btn primary" form="svc-req">Send request</button></>}>
          <form id="svc-req" className="col" onSubmit={send}>
            <p className="muted" style={{ margin: 0 }}>{req.summary}</p>
            <label className="f">Context <span className="hint">your use case, volume, timeline</span><textarea maxLength={1000} value={msg} onChange={(e) => setMsg(e.target.value)} /></label>
            {err && <div className="err">{err}</div>}
          </form>
        </Modal>
      )}
    </>
  )
}
