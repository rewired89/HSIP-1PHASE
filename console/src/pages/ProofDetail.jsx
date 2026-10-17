import { useState } from 'react'
import { Link, useParams } from 'react-router-dom'
import { useOrg } from '../auth.jsx'
import { download } from '../format.js'
import { CopyField, Gate, Modal, PageHeader, useLoad, useToast } from '../ui.jsx'
import ProofView from './ProofView.jsx'

function ShareModal({ proofId, onClose }) {
  const org = useOrg()
  const [days, setDays] = useState('30')
  const [link, setLink] = useState(null)
  const [err, setErr] = useState('')
  const make = async () => {
    setErr('')
    try { setLink(await org.post(`/proofs/${proofId}/shares`, { expires_days: days === 'never' ? null : Number(days) })) } catch (e) { setErr(e.message) }
  }
  return (
    <Modal title="Share this proof" onClose={onClose} footer={<button className="btn" onClick={onClose}>Done</button>}>
      {!link ? (
        <div className="col">
          <p className="muted">Anyone with the link can see this record's metadata and its verification result, without an account. They cannot see anything else in your organization.</p>
          <label className="f">Link expires
            <select value={days} onChange={(e) => setDays(e.target.value)}>
              <option value="7">in 7 days</option><option value="30">in 30 days</option><option value="90">in 90 days</option><option value="365">in 1 year</option><option value="never">never</option>
            </select>
          </label>
          {err && <div className="err">{err}</div>}
          <button className="btn primary" onClick={make}>Create link</button>
        </div>
      ) : (
        <div className="col">
          <CopyField value={link.url} label="Public verification link" />
          <div className="note">Revoke it any time from Records → this proof, or via the shares list.</div>
        </div>
      )}
    </Modal>
  )
}

export default function ProofDetail() {
  const { pid } = useParams()
  const org = useOrg()
  const toast = useToast()
  const st = useLoad(() => org.get(`/proofs/${pid}`), [org.id, pid])
  const [sharing, setSharing] = useState(false)
  const shares = useLoad(() => org.get('/shares'), [org.id, sharing])

  const revoke = async (id) => {
    try { await org.del(`/shares/${id}`); toast('Link revoked'); shares.reload() } catch (e) { toast(e.message, 'bad') }
  }

  return (
    <Gate state={st}>
      {(d) => {
        const mine = (shares.data?.shares || []).filter((s) => s.proof_id === pid && !s.revoked_at)
        return (
          <>
            <PageHeader title="Proof" sub={<>From stream <Link to={`/streams/${d.bundle.envelope.stream_id}`}>{d.stream_name}</Link> · <span className="mono">{pid}</span></>}>
              <button className="btn" onClick={() => download(`proof-${pid}.json`, JSON.stringify(d.bundle, null, 2))}>Download bundle</button>
              <button className="btn primary" onClick={() => setSharing(true)}>Share link</button>
            </PageHeader>
            <ProofView
              bundle={d.bundle}
              verification={d.verification}
              actions={mine.length > 0 && (
                <section className="card"><div className="card-h"><h2>Active share links</h2></div>
                  <div className="card-b col">
                    {mine.map((s) => (
                      <div className="row" key={s.id}>
                        <span className="grow small">{s.views} view{s.views === 1 ? '' : 's'} · {s.expires_at ? `expires ${new Date(s.expires_at).toLocaleDateString()}` : 'no expiry'}</span>
                        <button className="btn sm danger" onClick={() => revoke(s.id)}>Revoke</button>
                      </div>
                    ))}
                  </div>
                </section>
              )}
            />
            {sharing && <ShareModal proofId={pid} onClose={() => setSharing(false)} />}
          </>
        )
      }}
    </Gate>
  )
}
