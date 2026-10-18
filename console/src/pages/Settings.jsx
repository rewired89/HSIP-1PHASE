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
  const [cust, setCust] = useState({ verify_key: '', proof: '' })

  const enableCustody = async (e) => {
    e.preventDefault()
    if (!window.confirm('From now on your records are signed with your own key. This cannot be switched back to platform signing, and if you lose that key we cannot recover it. Continue?')) return
    try {
      await org.post('/key-custody', { verify_key: cust.verify_key.trim(), proof: cust.proof.trim() })
      toast('Your key is now registered'); setCust({ verify_key: '', proof: '' }); st.reload()
    } catch (x) { toast(x.message, 'bad') }
  }

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
            <Card title="Key custody">
              {d.custody?.mode === 'customer' ? (
                <div className="col">
                  <p style={{ margin: 0 }}><Badge kind="ok">You hold your signing key</Badge> New records are signed on your side. The platform checks each signature against the public key below and never sees your private key.</p>
                  <CopyField label="Your fingerprint" value={d.custody.fingerprint} />
                  <CopyField label="Your public key" value={d.custody.customer_key} />
                  <p className="small muted" style={{ margin: 0 }}>Records made before the switch stay valid under the platform key that signed them. The platform key still signs the activity log and evidence-pack manifests. Rotate your key by registering a new one below.</p>
                </div>
              ) : (
                <p className="muted" style={{ margin: 0 }}>Right now the platform signs your records with the organization key above. If you would rather hold the signing key yourself, so that even a breach of this platform cannot forge your records, switch below.</p>
              )}
              {org.isOwner ? (
                <form className="col" style={{ marginTop: 12 }} onSubmit={enableCustody}>
                  <p className="small" style={{ margin: 0 }}>1. On your own machine run <code>hsip-proof keygen &gt; hsip-key.json</code> (keep that file secret and backed up). 2. Run <code>hsip-proof register-proof {org.id} hsip-key.json</code>. 3. Paste the two values it prints.</p>
                  <input required placeholder="verify_key" value={cust.verify_key} onChange={(e) => setCust({ ...cust, verify_key: e.target.value })} aria-label="Public key" />
                  <input required placeholder="proof" value={cust.proof} onChange={(e) => setCust({ ...cust, proof: e.target.value })} aria-label="Registration proof" />
                  <div><button className="btn primary">{d.custody?.mode === 'customer' ? 'Rotate to this key' : 'Switch to my own key'}</button></div>
                </form>
              ) : <p className="small muted">Only an owner can change key custody.</p>}
              <p className="small muted" style={{ marginTop: 10 }}>Once switched, the console's “Send test proof” is turned off (the platform cannot sign for you); use an SDK with your key instead. Lose the key and you cannot record new proofs until you register another one.</p>
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
