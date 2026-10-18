import { useEffect, useState } from 'react'
import { Link } from 'react-router-dom'
import { api } from '../api.js'
import { money, fmtNum } from '../format.js'
import { Logo } from '../icons.jsx'
import { Card, useToast } from '../ui.jsx'
import { ServiceGrid } from './Enterprise.jsx'

export function PublicFrame({ children }) {
  return (
    <div>
      <nav className="land-nav" aria-label="Site">
        <Link to="/" className="brand"><Logo /> <span>HSIP</span></Link>
        <span className="grow" />
        <Link to="/verify">Verify a proof</Link>
        <Link to="/login">Sign in</Link>
        <Link to="/signup" className="btn primary">Start free</Link>
      </nav>
      <div style={{ padding: '8px max(16px, calc((100% - 1000px) / 2)) 64px' }}>{children}</div>
    </div>
  )
}

const FEATURES = [
  ['Tamper-evident by construction', 'Each record is signed and chained to the one before it. Editing, deleting or reordering anything breaks the chain in a way anyone can detect.'],
  ['Your data never leaves', 'You send a SHA-256 fingerprint, not the record. We cannot read, leak or be subpoenaed for what we never received.'],
  ['Independent verification', 'Auditors verify with an open-source tool, offline, with no account and no need to trust us. Our word is not part of the proof.'],
  ['Time you cannot backdate', 'Records are batched into signed Merkle trees and optionally committed to Bitcoin through OpenTimestamps.'],
  ['Evidence packs on demand', 'Export a signed, complete, verifiable record of a period for a regulator, a counterparty or a court, plus a printable report.'],
  ['Built for teams', 'Streams per system, scoped API keys, owner / admin / developer / auditor roles, and a signed audit trail of our own actions.'],
]

export default function Landing() {
  const [plans, setPlans] = useState([])
  const [services, setServices] = useState([])
  useEffect(() => { api.get('/v1/meta', { token: null }).then((m) => { setPlans(m.plans); setServices(m.services || []) }).catch(() => {}) }, [])
  return (
    <div>
      <nav className="land-nav" aria-label="Site">
        <Link to="/" className="brand"><Logo /> <span>HSIP</span></Link>
        <span className="grow" />
        <a href="#how">How it works</a>
        <a href="#pricing">Pricing</a>
        <a href="#enterprise">Enterprise</a>
        <Link to="/verify">Verify a proof</Link>
        <Link to="/login">Sign in</Link>
        <Link to="/signup" className="btn primary">Start free</Link>
      </nav>

      <header className="hero">
        <div>
          <span className="badge brand">For AI agents, automated decisions and regulated workflows</span>
          <h1 style={{ marginTop: 14 }}>Prove what your systems decided, and when.</h1>
          <p className="lead">HSIP turns every automated decision into a signed, chained, independently verifiable record. When a regulator, customer or court asks “how do you know?”, you hand them a proof instead of a log file.</p>
          <div className="row wrap">
            <Link to="/signup" className="btn primary" style={{ padding: '11px 20px' }}>Start free</Link>
            <a href="#contact" className="btn" style={{ padding: '11px 20px' }}>Talk to sales</a>
          </div>
          <p className="small muted" style={{ marginTop: 14 }}>Free plan includes {fmtNum(plans[0]?.proofs_per_month ?? 1000)} proofs a month. No card required.</p>
        </div>
        <div className="card" style={{ padding: 20 }} aria-hidden="true">
          <div className="verdict ok" style={{ marginBottom: 14 }}><div style={{ fontSize: 28 }}>✓</div><div><div className="big">Verified</div><div className="small">Authentic, unaltered, anchored</div></div></div>
          <dl className="kv small">
            <dt>Action</dt><dd>loan.decision</dd>
            <dt>Actor</dt><dd>underwriting-model-v3</dd>
            <dt>Subject</dt><dd>application-48211</dd>
            <dt>Recorded</dt><dd>2026-10-17 09:41:07 UTC</dd>
            <dt>Record hash</dt><dd><span className="hash">9f2c41ab…e07d11</span></dd>
            <dt>Batch</dt><dd>Merkle root signed · Bitcoin timestamp pending</dd>
          </dl>
        </div>
      </header>

      <section className="section alt" id="how">
        <h2>How it works</h2>
        <p className="lead">Three calls to get started. No data migration, no agents to install.</p>
        <div className="grid g3 steps">
          <div className="card step"><h3>1. Hash and send</h3><p className="muted">Your system hashes the decision record and POSTs the hash with an actor and an action. The content stays with you.</p></div>
          <div className="card step"><h3>2. We sign and chain</h3><p className="muted">The hash is stamped with the time, signed by your organization’s key, and linked to the previous record in its stream.</p></div>
          <div className="card step"><h3>3. Anyone can verify</h3><p className="muted">Share a link or export an evidence pack. Verification is a pure function that runs offline against open-source code.</p></div>
        </div>
      </section>

      <section className="section">
        <h2>What you get</h2>
        <p className="lead">The properties auditors ask for, built into the format rather than promised in a policy.</p>
        <div className="grid g3">
          {FEATURES.map(([t, d]) => <Card key={t}><h3 style={{ marginBottom: 6 }}>{t}</h3><p className="muted" style={{ margin: 0 }}>{d}</p></Card>)}
        </div>
      </section>

      <section className="section alt">
        <h2>Honest about the limits</h2>
        <p className="lead">A proof shows that a record existed in this exact form at a point in time, was signed by your key, and has not been changed, removed or reordered since. It does not show that the decision was correct, and it can only cover what you choose to record.</p>
      </section>

      <section className="section" id="pricing">
        <h2>Pricing</h2>
        <p className="lead">Start free. Move up when your volume or team does.</p>
        <div className="grid g4">
          {plans.map((p) => (
            <div key={p.id} className={`card price ${p.id === 'team' ? 'hot' : ''}`}>
              <h3>{p.name}</h3>
              <div className="amt">{money(p.price_usd_month)}{p.price_usd_month ? <span className="small muted"> /mo</span> : null}</div>
              <div className="small muted">{fmtNum(p.proofs_per_month)} proofs/mo · {fmtNum(p.streams)} streams · {fmtNum(p.seats)} seats</div>
              <ul>{p.highlights.map((h) => <li key={h}>{h}</li>)}</ul>
              <span className="grow" />
              {p.id === 'enterprise' ? <a href="#contact" className="btn">Contact sales</a> : <Link to="/signup" className={`btn ${p.id === 'team' ? 'primary' : ''}`}>Start with {p.name}</Link>}
            </div>
          ))}
        </div>
      </section>

      <section className="section alt" id="enterprise">
        <h2>Enterprise services</h2>
        <p className="lead">For teams that need their own deployment, larger volumes or hands-on help. Each item says plainly whether it exists today.</p>
        <ServiceGrid services={services} />
      </section>

      <section className="section" id="contact"><ContactForm /></section>
      <footer className="foot small">© {new Date().getFullYear()} Nyx Systems LLC · HSIP. Proof format and verifier are open: run <span className="mono">hsip-proof verify</span> on any bundle. <Link to="/operator">Operator</Link></footer>
    </div>
  )
}

function ContactForm() {
  const toast = useToast()
  const [f, setF] = useState({ email: '', company: '', message: '' })
  const [state, setState] = useState({ busy: false, done: false, error: '' })
  const submit = async (e) => {
    e.preventDefault()
    setState({ busy: true, done: false, error: '' })
    try {
      await api.post('/v1/leads', f, { token: null })
      setState({ busy: false, done: true, error: '' })
      toast('Thanks, we will be in touch')
    } catch (err) {
      setState({ busy: false, done: false, error: err.message })
    }
  }
  return (
    <div className="grid g2" style={{ alignItems: 'start' }}>
      <div>
        <h2>Talk to us</h2>
        <p className="lead">Dedicated or self-hosted deployment, custom volumes, security review, or help scoping what to record. Tell us about your use case.</p>
      </div>
      {state.done ? <div className="note">Thanks. We will reply to {f.email} shortly.</div> : (
        <form className="col" onSubmit={submit}>
          <label className="f">Work email<input type="email" required value={f.email} onChange={(e) => setF({ ...f, email: e.target.value })} /></label>
          <label className="f">Company<input required value={f.company} onChange={(e) => setF({ ...f, company: e.target.value })} /></label>
          <label className="f">What do you need to prove?<textarea value={f.message} onChange={(e) => setF({ ...f, message: e.target.value })} /></label>
          {state.error && <div className="err">{state.error}</div>}
          <button className="btn primary" disabled={state.busy}>Send</button>
        </form>
      )}
    </div>
  )
}
