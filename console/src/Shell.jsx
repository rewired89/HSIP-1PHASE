import { useEffect, useState } from 'react'
import { NavLink, useLocation, useNavigate } from 'react-router-dom'
import { useAuth } from './auth.jsx'
import Icon, { Logo } from './icons.jsx'
import { Badge, ErrorBoundary } from './ui.jsx'

const NAV = [
  ['Workspace', [
    ['/', 'Overview', 'home', true],
    ['/streams', 'Streams', 'stream'],
    ['/records', 'Records', 'list'],
    ['/evidence', 'Evidence', 'doc'],
    ['/permissions', 'Permissions', 'shield'],
    ['/verify', 'Verify', 'shield'],
    ['/activity', 'Audit trail', 'check'],
  ]],
  ['Developers', [
    ['/quickstart', 'Quickstart', 'code'],
    ['/api-keys', 'API keys', 'key'],
  ]],
  ['Organization', [
    ['/team', 'Team', 'users'],
    ['/billing', 'Plan & usage', 'card'],
    ['/enterprise', 'Enterprise', 'op'],
    ['/settings', 'Settings', 'cog'],
  ]],
]

function useTheme() {
  const [theme, setTheme] = useState(() => {
    try { return localStorage.getItem('hsip_prove_theme') || 'auto' } catch { return 'auto' }
  })
  useEffect(() => {
    const r = document.documentElement
    theme === 'auto' ? r.removeAttribute('data-theme') : r.setAttribute('data-theme', theme)
    try { localStorage.setItem('hsip_prove_theme', theme) } catch { /* storage blocked */ }
  }, [theme])
  return [theme, () => setTheme((t) => (t === 'auto' ? 'dark' : t === 'dark' ? 'light' : 'auto'))]
}

export default function Shell({ children }) {
  const { user, orgs, org, orgId, switchOrg, signOut } = useAuth()
  const [open, setOpen] = useState(false)
  const [theme, cycleTheme] = useTheme()
  const loc = useLocation()
  const nav = useNavigate()
  useEffect(() => setOpen(false), [loc.pathname])
  const hasOperator = (() => { try { return !!sessionStorage.getItem('hsip_operator') } catch { return false } })()

  return (
    <div className="shell">
      <aside className={`side ${open ? 'open' : ''}`} aria-label="Primary">
        <div className="brand"><Logo /><div>HSIP<small>Proof-of-record platform</small></div></div>
        {orgs.length > 1 ? (
          <select className="orgpick" value={orgId || ''} onChange={(e) => { switchOrg(e.target.value); nav('/') }} aria-label="Organization">
            {orgs.map((o) => <option key={o.id} value={o.id}>{o.name}</option>)}
          </select>
        ) : (
          <div className="orgpick" style={{ fontWeight: 600 }}>{org?.name}</div>
        )}
        <nav className="nav">
          {NAV.map(([group, items]) => (
            <div key={group}>
              <div className="nav-group">{group}</div>
              {items.map(([to, label, icon, end]) => (
                <NavLink key={to} to={to} end={!!end} className={({ isActive }) => (isActive ? 'active' : '')}>
                  <Icon name={icon} />{label}
                </NavLink>
              ))}
            </div>
          ))}
          {hasOperator && (
            <div>
              <div className="nav-group">Platform</div>
              <NavLink to="/operator"><Icon name="op" />Operator</NavLink>
            </div>
          )}
        </nav>
        <div className="side-foot">
          <div className="who truncate">{user?.name}</div>
          <div className="truncate" style={{ opacity: 0.7 }}>{user?.email}</div>
          <div className="row" style={{ marginTop: 6 }}>
            <button onClick={cycleTheme}>Theme: {theme}</button>
            <button className="right" onClick={() => signOut().then(() => nav('/'))}>Sign out</button>
          </div>
        </div>
      </aside>
      <div className="main">
        <div className="topbar">
          <button className="btn ghost menu-btn" onClick={() => setOpen((o) => !o)} aria-label="Open menu"><Icon name="menu" /></button>
          <div className="titles grow">
            <strong>{org?.name}</strong>
            <span className="sub"> · {org?.plan ? org.plan.charAt(0).toUpperCase() + org.plan.slice(1) : ''} plan</span>
          </div>
          <Badge kind="brand">{org?.role}</Badge>
        </div>
        <main className="content"><ErrorBoundary key={loc.pathname}>{children}</ErrorBoundary></main>
      </div>
    </div>
  )
}
