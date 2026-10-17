import { Component, createContext, useCallback, useContext, useEffect, useRef, useState } from 'react'
import Icon from './icons.jsx'
import { fmtNum } from './format.js'

/* ------------------------------------------------------------------ toasts */
const ToastCtx = createContext(() => {})
export const useToast = () => useContext(ToastCtx)

export function ToastProvider({ children }) {
  const [items, setItems] = useState([])
  const push = useCallback((msg, kind = 'ok') => {
    const id = Math.random()
    setItems((x) => [...x, { id, msg, kind }])
    setTimeout(() => setItems((x) => x.filter((i) => i.id !== id)), 3600)
  }, [])
  return (
    <ToastCtx.Provider value={push}>
      {children}
      <div className="toasts" role="status" aria-live="polite">
        {items.map((t) => <div key={t.id} className={`toast ${t.kind === 'bad' ? 'bad' : ''}`}>{t.msg}</div>)}
      </div>
    </ToastCtx.Provider>
  )
}

/* ----------------------------------------------------------------- loading */
export function useLoad(fn, deps = []) {
  const [state, set] = useState({ data: null, error: null, loading: true })
  const seq = useRef(0)
  const run = useCallback(async () => {
    const my = ++seq.current
    set((s) => ({ ...s, loading: true, error: null }))
    try {
      const data = await fn()
      if (my === seq.current) set({ data, error: null, loading: false })
    } catch (e) {
      if (my === seq.current) set({ data: null, error: e, loading: false })
    }
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, deps)
  useEffect(() => { run() }, [run])
  return { ...state, reload: run }
}

export const Spinner = () => <span className="spinner" role="progressbar" aria-label="Loading" />
export const Loading = () => <div className="center"><Spinner /></div>

export function Gate({ state, children, empty }) {
  if (state.loading && !state.data) return <Loading />
  if (state.error) return <div className="err">{state.error.message}</div>
  if (empty && empty(state.data)) return null
  return typeof children === 'function' ? children(state.data) : children
}

/** A crash in one page must not blank the whole console. */
export class ErrorBoundary extends Component {
  state = { error: null }
  static getDerivedStateFromError(error) { return { error } }
  componentDidCatch(error) { console.error('page crashed:', error) }
  render() {
    if (!this.state.error) return this.props.children
    return (
      <div className="card" style={{ padding: 24 }}>
        <h2>Something went wrong on this page</h2>
        <p className="muted">Your data is safe. Reload the page, or go back to the overview.</p>
        <a className="btn" href="/">Back to overview</a>
      </div>
    )
  }
}

/* ----------------------------------------------------------------- layout */
export function PageHeader({ title, sub, children }) {
  return (
    <div className="row wrap" style={{ marginBottom: 20 }}>
      <div className="grow">
        <h1>{title}</h1>
        {sub && <div className="muted" style={{ marginTop: 4 }}>{sub}</div>}
      </div>
      <div className="row wrap">{children}</div>
    </div>
  )
}

export function Card({ title, actions, children, flush, className = '' }) {
  return (
    <section className={`card ${flush ? 'flush' : ''} ${className}`}>
      {(title || actions) && (
        <div className="card-h">
          <h2 className="grow">{title}</h2>
          {actions}
        </div>
      )}
      <div className="card-b">{children}</div>
    </section>
  )
}

export const Stat = ({ label, value, detail }) => (
  <div className="card stat">
    <div className="k">{label}</div>
    <div className="v">{value}</div>
    {detail && <div className="d">{detail}</div>}
  </div>
)

export const Empty = ({ title, children }) => (
  <div className="empty">
    <h3>{title}</h3>
    <div>{children}</div>
  </div>
)

export const Badge = ({ kind = '', children }) => <span className={`badge ${kind}`}>{children}</span>

export function AnchorBadge({ anchored }) {
  return anchored ? <Badge kind="ok"><span className="dot" />Anchored</Badge> : <Badge kind="warn"><span className="dot" />Awaiting anchor</Badge>
}

/* ------------------------------------------------------------------- modal */
export function Modal({ title, onClose, children, footer, wide }) {
  useEffect(() => {
    const h = (e) => e.key === 'Escape' && onClose()
    document.addEventListener('keydown', h)
    return () => document.removeEventListener('keydown', h)
  }, [onClose])
  return (
    <div className="scrim" onMouseDown={(e) => e.target === e.currentTarget && onClose()}>
      <div className={`modal ${wide ? 'wide' : ''}`} role="dialog" aria-modal="true" aria-label={title}>
        <div className="mh"><h2 className="grow">{title}</h2><button className="btn ghost sm" onClick={onClose} aria-label="Close">✕</button></div>
        <div className="mb">{children}</div>
        {footer && <div className="mf">{footer}</div>}
      </div>
    </div>
  )
}

/* --------------------------------------------------------------- copy field */
export function CopyField({ value, label }) {
  const toast = useToast()
  const copy = async () => {
    try {
      await navigator.clipboard.writeText(value)
      toast('Copied')
    } catch {
      toast('Copy failed: select the text manually', 'bad')
    }
  }
  return (
    <div>
      {label && <div className="small muted" style={{ marginBottom: 4 }}>{label}</div>}
      <div className="copyfield">
        <div className="val">{value}</div>
        <button className="btn" onClick={copy} aria-label="Copy"><Icon name="copy" size={16} /></button>
      </div>
    </div>
  )
}

export function CodeBlock({ code }) {
  const toast = useToast()
  return (
    <div style={{ position: 'relative' }}>
      <button
        className="btn sm"
        style={{ position: 'absolute', top: 8, right: 8 }}
        onClick={() => navigator.clipboard.writeText(code).then(() => toast('Copied'), () => toast('Copy failed', 'bad'))}
      >Copy</button>
      <pre className="code">{code}</pre>
    </div>
  )
}

export function Tabs({ tabs, value, onChange }) {
  return (
    <div className="tabs" role="tablist">
      {tabs.map(([k, label]) => (
        <button key={k} role="tab" aria-selected={value === k} className={value === k ? 'on' : ''} onClick={() => onChange(k)}>{label}</button>
      ))}
    </div>
  )
}

/* ----------------------------------------------------------------- charts */
export function BarChart({ counts, labels, height = 150 }) {
  const w = 600
  const max = Math.max(1, ...counts)
  const bw = w / counts.length
  return (
    <svg className="chart" viewBox={`0 0 ${w} ${height}`} preserveAspectRatio="none" role="img" aria-label="Activity chart">
      {counts.map((c, i) => {
        const h = Math.round((c / max) * (height - 22))
        return (
          <g key={i}>
            <rect x={i * bw + 2} y={height - 18 - h} width={Math.max(2, bw - 4)} height={Math.max(c ? 2 : 0, h)} rx="2"><title>{`${labels[i]}: ${fmtNum(c)}`}</title></rect>
          </g>
        )
      })}
      <text x="2" y={height - 4}>{labels[0]}</text>
      <text x={w - 2} y={height - 4} textAnchor="end">{labels[labels.length - 1]}</text>
    </svg>
  )
}

export function Meter({ used, limit }) {
  const unlimited = limit >= 9e18
  const pct = unlimited ? 0 : Math.min(100, (used / Math.max(1, limit)) * 100)
  return (
    <div>
      <div className="row small" style={{ marginBottom: 6 }}>
        <span className="grow">{fmtNum(used)} used</span>
        <span className="muted">of {fmtNum(limit)}</span>
      </div>
      <div className={`bar ${pct >= 100 ? 'bad' : pct >= 80 ? 'warn' : ''}`}><i style={{ width: `${pct}%` }} /></div>
    </div>
  )
}

/* ----------------------------------------------------------- verification */
export function Checks({ checks }) {
  const names = {
    content_hash: 'Content is unaltered',
    signature: 'Signed by the organization',
    format: 'Recognized proof format',
    batch_inclusion: 'Included in an anchored batch',
    anchor_signature: 'Batch signed by the anchor key',
    manifest_hash: 'Export manifest intact',
    manifest_signature: 'Export signed by the organization',
    manifest_matches_proofs: 'No records removed or added',
    proofs_valid: 'Every record verifies',
    single_issuer: 'One issuing key throughout',
    hash_chain: 'Unbroken hash chain',
  }
  return (
    <div className="checks">
      {checks.map((c) => (
        <div className="check" key={c.name}>
          <div className={`ic ${c.ok === true ? 'ok' : c.ok === false ? 'bad' : 'na'}`} aria-label={c.ok === true ? 'passed' : c.ok === false ? 'failed' : 'not applicable'}>{c.ok === true ? '✓' : c.ok === false ? '✕' : '–'}</div>
          <div>
            <div className="nm">{names[c.name] || c.name}</div>
            <div className="small muted">{c.detail}</div>
          </div>
        </div>
      ))}
    </div>
  )
}

export function Verdict({ valid, anchored, kind = 'proof' }) {
  return (
    <div className={`verdict ${valid ? 'ok' : 'bad'}`} role="status">
      <div style={{ fontSize: 30 }}>{valid ? '✓' : '✕'}</div>
      <div>
        <div className="big">{valid ? (kind === 'proof' ? 'Verified' : 'Evidence verified') : 'Verification failed'}</div>
        <div className="small">
          {valid
            ? kind === 'proof'
              ? anchored ? 'Authentic, unaltered, and included in a signed, timestamped batch.' : 'Authentic and unaltered. Batch anchoring is still pending.'
              : 'Every record is authentic and the chain is complete.'
            : 'This record or export does not match what was originally signed. Do not rely on it.'}
        </div>
      </div>
    </div>
  )
}
