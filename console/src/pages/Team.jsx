import { useState } from 'react'
import { useAuth, useOrg } from '../auth.jsx'
import { ago, fmtDate } from '../format.js'
import { Badge, Card, CopyField, Gate, Modal, PageHeader, useLoad, useToast } from '../ui.jsx'

const ROLES = [
  ['owner', 'Owner', 'Everything, including billing requests and other owners'],
  ['admin', 'Admin', 'Manage streams, keys and people'],
  ['developer', 'Developer', 'Create streams, record and read proofs'],
  ['auditor', 'Auditor', 'Read-only, can export evidence'],
]

export default function Team() {
  const org = useOrg()
  const { user } = useAuth()
  const toast = useToast()
  const st = useLoad(() => org.get('/members'), [org.id])
  const [inviting, setInviting] = useState(false)
  const [f, setF] = useState({ email: '', role: 'developer' })
  const [err, setErr] = useState('')
  const [fresh, setFresh] = useState(null)

  const invite = async (e) => {
    e.preventDefault()
    setErr('')
    try { setFresh(await org.post('/invites', f)); setInviting(false); setF({ email: '', role: 'developer' }); st.reload() } catch (x) { setErr(x.message) }
  }
  const act = async (fn, msg) => { try { await fn(); toast(msg); st.reload() } catch (x) { toast(x.message, 'bad') } }

  return (
    <>
      <PageHeader title="Team" sub="Who can see and change what in this organization.">
        {org.isAdmin && <button className="btn primary" onClick={() => setInviting(true)}>Invite person</button>}
      </PageHeader>
      <Gate state={st}>
        {(d) => (
          <div className="col" style={{ gap: 16 }}>
            <Card flush>
              <div className="table-wrap">
                <table className="t">
                  <thead><tr><th>Person</th><th>Role</th><th>Joined</th><th>Last sign-in</th><th /></tr></thead>
                  <tbody>
                    {d.members.map((m) => (
                      <tr key={m.user_id}>
                        <td><strong>{m.name}</strong>{m.user_id === user.id && <> <Badge>you</Badge></>}<div className="small muted">{m.email}</div></td>
                        <td>
                          {org.isAdmin ? (
                            <select className="inline-input" value={m.role} aria-label={`Role for ${m.email}`}
                              onChange={(e) => act(() => org.patch(`/members/${m.user_id}`, { role: e.target.value }), 'Role updated')}>
                              {ROLES.map(([v, l]) => <option key={v} value={v} disabled={v === 'owner' && !org.isOwner}>{l}</option>)}
                            </select>
                          ) : <Badge>{m.role}</Badge>}
                        </td>
                        <td className="small">{fmtDate(m.joined_at)}</td>
                        <td className="small">{ago(m.last_login_at)}</td>
                        <td style={{ textAlign: 'right' }}>
                          {(org.isAdmin || m.user_id === user.id) && (
                            <button className="btn sm danger" onClick={() => window.confirm(m.user_id === user.id ? 'Leave this organization?' : `Remove ${m.email}?`) && act(() => org.del(`/members/${m.user_id}`), 'Removed')}>
                              {m.user_id === user.id ? 'Leave' : 'Remove'}
                            </button>
                          )}
                        </td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </div>
            </Card>
            {d.invites.length > 0 && (
              <Card title="Pending invitations" flush>
                <table className="t">
                  <tbody>
                    {d.invites.map((i) => (
                      <tr key={i.id}>
                        <td>{i.email}</td><td><Badge>{i.role}</Badge></td>
                        <td className="small muted">expires {new Date(i.expires_at).toLocaleDateString()}</td>
                        <td style={{ textAlign: 'right' }}>{org.isAdmin && <button className="btn sm" onClick={() => act(() => org.del(`/invites/${i.id}`), 'Invitation cancelled')}>Cancel</button>}</td>
                      </tr>
                    ))}
                  </tbody>
                </table>
              </Card>
            )}
            <Card title="Roles">
              <div className="col">{ROLES.map(([v, l, desc]) => <div className="row" key={v}><Badge kind={v === 'owner' ? 'brand' : ''}>{l}</Badge><span className="muted">{desc}</span></div>)}</div>
            </Card>
          </div>
        )}
      </Gate>
      {inviting && (
        <Modal title="Invite person" onClose={() => setInviting(false)} footer={<><button className="btn" onClick={() => setInviting(false)}>Cancel</button><button className="btn primary" form="invite">Create invitation</button></>}>
          <form id="invite" className="col" onSubmit={invite}>
            <label className="f">Email<input type="email" required autoFocus value={f.email} onChange={(e) => setF({ ...f, email: e.target.value })} /></label>
            <label className="f">Role
              <select value={f.role} onChange={(e) => setF({ ...f, role: e.target.value })}>
                {ROLES.map(([v, l, d]) => <option key={v} value={v} disabled={v === 'owner' && !org.isOwner}>{l}: {d}</option>)}
              </select>
            </label>
            {err && <div className="err">{err}</div>}
          </form>
        </Modal>
      )}
      {fresh && (
        <Modal title="Invitation ready" onClose={() => setFresh(null)} footer={<button className="btn primary" onClick={() => setFresh(null)}>Done</button>}>
          <div className="col">
            <div className="note warn">{fresh.note}</div>
            <CopyField value={fresh.invite_url} label={`Link for ${fresh.email} (${fresh.role})`} />
          </div>
        </Modal>
      )}
    </>
  )
}
