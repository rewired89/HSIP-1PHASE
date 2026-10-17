import { useParams } from 'react-router-dom'
import { api } from '../api.js'
import { Link } from 'react-router-dom'
import { Gate, PageHeader, useLoad } from '../ui.jsx'
import ProofView, { Fact } from './ProofView.jsx'

export default function SharedProof() {
  const { token } = useParams()
  const st = useLoad(() => api.get(`/v1/public/shares/${token}`, { token: null }), [token])
  return (
    <Gate state={st}>
      {(d) => (
        <>
          <PageHeader title="Shared proof" sub={`Issued by ${d.org_name}. This page re-verifies the record every time it is opened.`} />
          <ProofView
            bundle={d.bundle}
            verification={d.verification}
            extra={<Fact k="Issuer key">Fingerprint <span className="hash">{d.org_fingerprint}</span></Fact>}
            actions={
              <div className="note">
                Do not take our word for it. Download the bundle from the “Proof bundle” tab and run <code>hsip-proof verify proof.json</code> on your own machine, or paste it into the <Link to="/verify">public verifier</Link>. To confirm the issuer, compare the key fingerprint above with one {d.org_name} gives you through another channel.
              </div>
            }
          />
        </>
      )}
    </Gate>
  )
}
