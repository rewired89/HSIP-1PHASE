import { useEffect, useState } from 'react'
import { Link, Navigate, useNavigate, useParams } from 'react-router-dom'
import { api } from '../api.js'
import { useAuth } from '../auth.jsx'
import { Logo } from '../icons.jsx'
import { Loading } from '../ui.jsx'

function Frame({ title, sub, children, foot }) {
  return (
    <div className="auth">
      <div className="auth-art">
        <div className="row" style={{ gap: 12 }}><Logo size={36} /><strong style={{ fontSize: 18 }}>HSIP</strong></div>
        <h1>Evidence that holds up when someone asks, “how do we know?”</h1>
        <p>Every decision your systems make becomes a signed, hash-chained, independently verifiable record. We hold fingerprints, never your data.</p>
        <ul>
          <li>Tamper-evident: edits, deletions and reordering are detectable</li>
          <li>Verifiable offline by auditors, with no account and no trust in us</li>
          <li>Exportable evidence packs for regulators and counterparties</li>
        </ul>
      </div>
      <div className="auth-form">
        <div style={{ width: '100%', maxWidth: 380 }}>
          <h1 style={{ marginBottom: 6 }}>{title}</h1>
          <p className="muted" style={{ marginBottom: 18 }}>{sub}</p>
          {children}
          <div className="small muted" style={{ marginTop: 18 }}>{foot}</div>
        </div>
      </div>
    </div>
  )
}

function useSubmit(fn) {
  const [busy, setBusy] = useState(false)
  const [error, setError] = useState('')
  return {
    busy,
    error,
    run: async (e) => {
      e?.preventDefault()
      setBusy(true)
      setError('')
      try { await fn() } catch (err) { setError(err.message) } finally { setBusy(false) }
    },
  }
}

export function Login() {
  const { status, signIn } = useAuth()
  const nav = useNavigate()
  const [f, setF] = useState({ email: '', password: '' })
  const s = useSubmit(async () => { signIn(await api.post('/v1/auth/login', f, { token: null })); nav('/') })
  if (status === 'in') return <Navigate to="/" replace />
  return (
    <Frame title="Sign in" sub="Welcome back." foot={<>New here? <Link to="/signup">Create an organization</Link><br />Running HSIP? <Link to="/operator">Admin login</Link> · <Link to="/enterprise">See Enterprise options</Link></>}>
      <form className="col" onSubmit={s.run}>
        <label className="f">Work email<input type="email" autoComplete="username" required value={f.email} onChange={(e) => setF({ ...f, email: e.target.value })} /></label>
        <label className="f">Password<input type="password" autoComplete="current-password" required value={f.password} onChange={(e) => setF({ ...f, password: e.target.value })} /></label>
        {s.error && <div className="err">{s.error}</div>}
        <button className="btn primary" disabled={s.busy}>{s.busy ? 'Signing in…' : 'Sign in'}</button>
      </form>
    </Frame>
  )
}

export function Signup() {
  const { status, signIn } = useAuth()
  const nav = useNavigate()
  const [meta, setMeta] = useState(null)
  const [f, setF] = useState({ name: '', org_name: '', email: '', password: '' })
  useEffect(() => { api.get('/v1/meta', { token: null }).then(setMeta).catch(() => setMeta({ signup_open: true })) }, [])
  const s = useSubmit(async () => { signIn(await api.post('/v1/auth/signup', f, { token: null })); nav('/') })
  if (status === 'in') return <Navigate to="/" replace />
  if (!meta) return <Loading />
  if (!meta.signup_open) {
    return (
      <Frame title="Invitation only" sub="Self-service sign-up is closed on this deployment." foot={<>Already have an account? <Link to="/login">Sign in</Link></>}>
        <div className="note">Ask your administrator for an invitation link.</div>
      </Frame>
    )
  }
  return (
    <Frame title="Create your organization" sub="Free to start. No payment details needed." foot={<>Already have an account? <Link to="/login">Sign in</Link></>}>
      <form className="col" onSubmit={s.run}>
        <label className="f">Your name<input required autoComplete="name" value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} /></label>
        <label className="f">Company<input required value={f.org_name} onChange={(e) => setF({ ...f, org_name: e.target.value })} /></label>
        <label className="f">Work email<input type="email" required autoComplete="username" value={f.email} onChange={(e) => setF({ ...f, email: e.target.value })} /></label>
        <label className="f">Password<input type="password" required minLength={10} autoComplete="new-password" value={f.password} onChange={(e) => setF({ ...f, password: e.target.value })} /><span className="hint">At least 10 characters.</span></label>
        {s.error && <div className="err">{s.error}</div>}
        <button className="btn primary" disabled={s.busy}>{s.busy ? 'Creating…' : 'Create organization'}</button>
      </form>
    </Frame>
  )
}

export function Invite() {
  const { token } = useParams()
  const { signIn } = useAuth()
  const nav = useNavigate()
  const [inv, setInv] = useState(null)
  const [bad, setBad] = useState('')
  const [f, setF] = useState({ name: '', password: '' })
  useEffect(() => { api.get(`/v1/auth/invite/${token}`, { token: null }).then(setInv).catch((e) => setBad(e.message)) }, [token])
  const s = useSubmit(async () => { signIn(await api.post('/v1/auth/accept-invite', { token, ...f }, { token: null })); nav('/') })
  if (bad) return <Frame title="Invitation unavailable" sub={bad} foot={<Link to="/login">Go to sign in</Link>}><div /></Frame>
  if (!inv) return <Loading />
  return (
    <Frame title={`Join ${inv.org_name}`} sub={`You were invited as ${inv.role}. Account: ${inv.email}`} foot={<Link to="/login">Back to sign in</Link>}>
      <form className="col" onSubmit={s.run}>
        {!inv.has_account && <label className="f">Your name<input required value={f.name} onChange={(e) => setF({ ...f, name: e.target.value })} /></label>}
        <label className="f">{inv.has_account ? 'Your existing password' : 'Choose a password'}
          <input type="password" required minLength={inv.has_account ? 1 : 10} value={f.password} onChange={(e) => setF({ ...f, password: e.target.value })} />
        </label>
        {s.error && <div className="err">{s.error}</div>}
        <button className="btn primary" disabled={s.busy}>{s.busy ? 'Joining…' : 'Accept invitation'}</button>
      </form>
    </Frame>
  )
}
