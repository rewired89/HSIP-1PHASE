import { createContext, useCallback, useContext, useEffect, useMemo, useState } from 'react'
import { api, setUnauthorizedHandler, tokenStore } from './api.js'

const Ctx = createContext(null)
export const useAuth = () => useContext(Ctx)

const ORG_KEY = 'hsip_prove_org'
const readOrg = () => { try { return localStorage.getItem(ORG_KEY) } catch { return null } }

export function AuthProvider({ children }) {
  const [state, setState] = useState({ status: tokenStore.get() ? 'loading' : 'out', user: null, orgs: [], orgId: null })

  const apply = useCallback((user, orgs, prefer) => {
    const saved = prefer || readOrg()
    const orgId = orgs.find((o) => o.id === saved)?.id || orgs[0]?.id || null
    try { orgId && localStorage.setItem(ORG_KEY, orgId) } catch { /* storage blocked */ }
    setState({ status: 'in', user, orgs, orgId })
  }, [])

  const signOut = useCallback(() => {
    tokenStore.set(null)
    setState({ status: 'out', user: null, orgs: [], orgId: null })
  }, [])

  useEffect(() => {
    setUnauthorizedHandler(signOut)
    if (!tokenStore.get()) return
    api.get('/v1/auth/me').then((r) => apply(r.user, r.orgs)).catch(signOut)
  }, [apply, signOut])

  const value = useMemo(
    () => ({
      ...state,
      org: state.orgs.find((o) => o.id === state.orgId) || null,
      signIn(r) { tokenStore.set(r.token); apply(r.user, r.orgs, r.org_id) },
      signOut: async () => { try { await api.post('/v1/auth/logout') } catch { /* already invalid */ } signOut() },
      switchOrg(id) { try { localStorage.setItem(ORG_KEY, id) } catch { /* ignore */ } setState((s) => ({ ...s, orgId: id })) },
      async refresh() { const r = await api.get('/v1/auth/me'); apply(r.user, r.orgs, state.orgId) },
    }),
    [state, apply, signOut],
  )
  return <Ctx.Provider value={value}>{children}</Ctx.Provider>
}

/** Org-scoped API helpers, bound to the current organization. */
export function useOrg() {
  const { orgId, org } = useAuth()
  return useMemo(() => {
    const base = `/v1/orgs/${orgId}`
    return {
      id: orgId,
      org,
      role: org?.role,
      isAdmin: org?.role === 'owner' || org?.role === 'admin',
      canWrite: org?.role !== 'auditor',
      isOwner: org?.role === 'owner',
      get: (p, o) => api.get(base + p, o),
      post: (p, b, o) => api.post(base + p, b, o),
      patch: (p, b, o) => api.patch(base + p, b, o),
      del: (p, o) => api.del(base + p, o),
      path: (p) => base + p,
    }
  }, [orgId, org])
}
