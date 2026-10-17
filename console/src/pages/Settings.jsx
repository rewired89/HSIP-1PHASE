import { useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { api } from '../api.js'
import { useAuth, useOrg } from '../auth.jsx'
import { fmtDate } from '../format.js'
import { Badge, Card, CopyField, Gate, PageHeader, useLoad, useToast } from '../ui.jsx'

export default function Settings() {
  const org = useOrg()
  const { refresh, switchOrg } = useAuth()
  const nav = useNavigate()
  const toast = useToast()
  const st = useLoad(() => org.get(''), [org.id])
  const [name, setName] = useState(null)
  const [newOrg, setNewOrg] = useState('')

  const rename = async (e) => {
    e.preventDefault()
    try { await org.patch('', { name }); toast('Saved'); setName(null); await refresh(); st.reload() } catch (x) { toast(x.message, 'bad') }
  }
  const create = async (e) => {
    e.preventDefault()
    try {
      const r = await api.post('/v1/orgs', { name: newOrg })
      await refresh()
      switchOrg(r.org_id)
      toast('Organization created')
      nav('/')
    } catch (x) { toast(x.message, 'bad') }
  }

  return (
    <>
      <PageHeader title="Settings" sub="Organization identity and signing keys." />
      <Gate state={st}>
        {(d) => (
          <div className="col" style={{ gap: 16 }}>
            <Card title="Organization">
              <form className="row wrap" onSubmit={rename}>
                <input style={{ maxWidth: 360 }} value={name ?? d.org.name} disabled={!org.isAdmin} onChange={(e) => setName(e.target.value)} aria-label="Organization name" />
                {org.isAdmin && <button className="btn primary" disabled={name === null || name === d.org.name}>Save</button>}
                <span className="muted small">Created {fmtDate(d.org.created_at)} · <Badge>{d.org.status}</Badge></span>
              </form>
            </Card>
            <Card title="Signing identity">
              <div className="col">
                <p className="muted" style={{ margin: 0 }}>Every proof is signed with this organization key. Share the fingerprint with auditors through a channel other than this site, so they can confirm a bundle really came from you.</p>
                <CopyField label="Fingerprint" value={d.org.fingerprint} />
                <CopyField label="Public verify key (Ed25519, base64)" value={d.org.verify_key} />
                <CopyField label="Published key page" value={`${window.location.origin}/v1/public/orgs/${d.org.slug}`} />
              </div>
            </Card>
            <Card title="Anchoring">
              <dl className="kv">
                <dt>Batch signing key</dt><dd className="mono small" style={{ wordBreak: 'break-all' }}>{d.anchor_verify_key}</dd>
                <dt>Bitcoin timestamps</dt><dd>{d.ots_enabled ? <Badge kind="ok">OpenTimestamps enabled</Badge> : <Badge>Not enabled on this deployment</Badge>}</dd>
              </dl>
              <p className="small muted" style={{ marginTop: 10 }}>Records are batched about every minute into a Merkle tree whose root is signed by the platform's anchor key{d.ots_enabled ? ' and committed to Bitcoin via OpenTimestamps calendars' : ''}.</p>
            </Card>
            <Card title="Start another organization">
              <form className="row wrap" onSubmit={create}>
                <input style={{ maxWidth: 360 }} required minLength={2} placeholder="Organization name" value={newOrg} onChange={(e) => setNewOrg(e.target.value)} aria-label="New organization name" />
                <button className="btn">Create</button>
              </form>
            </Card>
          </div>
        )}
      </Gate>
    </>
  )
}
