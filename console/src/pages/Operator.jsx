import { useMemo, useState } from 'react'
import { api } from '../api.js'
import { ago, download, fmtDay, fmtNum } from '../format.js'
import { Badge, Card, Gate, PageHeader, Stat, useLoad, useToast } from '../ui.jsx'

const KEY = 'hsip_operator'
const stored = () => { try { return sessionStorage.getItem(KEY) || '' } catch { return '' } }

function exportCsv(orgs) {
  const q = (v) => `"${String(v ?? '').replace(/"/g, '""')}"`
  const rows = [['organization', 'owner_email', 'plan', 'proofs_this_month', 'proofs_total', 'streams', 'members', 'last_proof', 'signed_up']]
  orgs.forEach((o) => rows.push([o.name, o.owner_email, o.plan, o.proofs_month, o.proofs_total, o.streams, o.members, o.last_proof_at ? new Date(o.last_proof_at).toISOString() : '', new Date(o.created_at).toISOString()]))
  download('hsip-prospects.csv', rows.map((r) => r.map(q).join(',')).join('\n'), 'text/csv')
}

function Console({ opKey, onLock }) {
  const toast = useToast()
  const opts = { token: opKey }
  const st = useLoad(async () => {
    const [ov, leads] = await Promise.all([api.get('/v1/operator/overview', opts), api.get('/v1/operator/leads', opts)])
    return { ov, leads: leads.leads }
  }, [opKey])

  const [onlyFree, setOnlyFree] = useState(false)
  const act = async (fn, msg) => { try { await fn(); toast(msg); st.reload() } catch (e) { toast(e.message, 'bad') } }

  return (
    <Gate state={st}>
      {({ ov, leads }) => (
        <div className="col" style={{ gap: 16 }}>
          <div className="grid g4">
            <Stat label="Organizations" value={fmtNum(ov.totals.orgs)} detail={`${fmtNum(ov.totals.users)} users`} />
            <Stat label="Proofs stored" value={fmtNum(ov.totals.proofs)} detail={`${fmtNum(ov.totals.unanchored)} awaiting anchor`} />
            <Stat label="Anchor batches" value={fmtNum(ov.totals.anchors)} detail={ov.ots_enabled ? `${ov.totals.ots_pending} pending · ${ov.totals.ots_unreachable} retrying` : 'OpenTimestamps off'} />
            <Stat label="Leads" value={fmtNum(ov.totals.leads)} />
          </div>
          <Card
            title="Customers and prospects"
            flush
            actions={
              <div className="row">
                <label className="row small"><input type="checkbox" style={{ width: 'auto' }} checked={onlyFree} onChange={(e) => setOnlyFree(e.target.checked)} /> Free plan only</label>
                <button className="btn sm" onClick={() => exportCsv(ov.orgs)}>Download CSV</button>
              </div>
            }
          >
            <div className="table-wrap">
              <table className="t">
                <thead><tr><th>Organization</th><th>Owner</th><th>Plan</th><th>This month</th><th>Total</th><th>Last proof</th><th>Streams</th><th>Members</th><th>Since</th><th>Status</th></tr></thead>
                <tbody>
                  {ov.orgs.filter((o) => !onlyFree || o.plan === 'free').map((o) => (
                    <tr key={o.id}>
                      <td><strong>{o.name}</strong><div className="small muted mono">{o.slug}</div></td>
                      <td className="small">{o.owner_email || '–'}</td>
                      <td>
                        <select className="inline-input" value={o.plan} aria-label={`Plan for ${o.name}`} onChange={(e) => act(() => api.post(`/v1/operator/orgs/${o.id}/plan`, { plan: e.target.value }, opts), 'Plan updated')}>
                          {['free', 'team', 'business', 'enterprise'].map((p) => <option key={p}>{p}</option>)}
                        </select>
                      </td>
                      <td>{fmtNum(o.proofs_month)}</td><td>{fmtNum(o.proofs_total)}</td><td className="small">{o.last_proof_at ? ago(o.last_proof_at) : 'never'}</td><td>{o.streams}</td><td>{o.members}</td>
                      <td className="small">{fmtDay(o.created_at)}</td>
                      <td>
                        <button className={`btn sm ${o.status === 'suspended' ? 'danger' : ''}`} onClick={() => act(() => api.post(`/v1/operator/orgs/${o.id}/status`, { status: o.status === 'active' ? 'suspended' : 'active' }, opts), o.status === 'active' ? 'Suspended' : 'Reactivated')}>
                          {o.status === 'active' ? 'Active' : 'Suspended'}
                        </button>
                      </td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          </Card>
          <Card title="Leads & plan requests" flush>
            {leads.length === 0 ? <div className="card-b muted">None yet.</div> : (
              <div className="table-wrap">
                <table className="t">
                  <thead><tr><th>When</th><th>Type</th><th>Company</th><th>Email</th><th>Message</th></tr></thead>
                  <tbody>{leads.map((l) => <tr key={l.id}><td className="nowrap">{ago(l.created_at)}</td><td><Badge kind={l.kind === 'plan_request' || l.kind === 'service_request' ? 'brand' : l.kind === 'signup' ? 'ok' : ''}>{l.kind.replace('_', ' ')}</Badge></td><td>{l.company}</td><td>{l.email}</td><td style={{ maxWidth: 360 }}>{l.message}</td></tr>)}</tbody>
                </table>
              </div>
            )}
          </Card>
          <div><button className="btn sm" onClick={onLock}>Lock operator console</button></div>
        </div>
      )}
    </Gate>
  )
}

export default function Operator() {
  const [key, setKey] = useState(stored())
  const [input, setInput] = useState('')
  const unlock = (e) => {
    e.preventDefault()
    try { sessionStorage.setItem(KEY, input) } catch { /* storage blocked */ }
    setKey(input)
  }
  const lock = () => { try { sessionStorage.removeItem(KEY) } catch { /* ignore */ } setKey(''); setInput('') }
  return (
    <>
      <PageHeader title="Admin console" sub="Platform-wide view for the team running HSIP: customers, free-plan prospects, leads and service requests. Not visible to customers." />
      {key ? <Console opKey={key} onLock={lock} /> : (
        <Card>
          <form className="col" style={{ maxWidth: 420 }} onSubmit={unlock}>
            <label className="f">Operator key<input type="password" required autoFocus value={input} onChange={(e) => setInput(e.target.value)} /><span className="hint">The HSP_OPERATOR_KEY value. Kept in this tab's session only.</span></label>
            <button className="btn primary">Unlock</button>
          </form>
        </Card>
      )}
    </>
  )
}
