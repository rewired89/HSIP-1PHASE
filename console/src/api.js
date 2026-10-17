// Single fetch wrapper. The session token lives in localStorage; losing it
// (private window, cleared storage) just signs the user out.
const KEY = 'hsip_prove_token'

export const tokenStore = {
  get() {
    try { return localStorage.getItem(KEY) } catch { return null }
  },
  set(t) {
    try { t ? localStorage.setItem(KEY, t) : localStorage.removeItem(KEY) } catch { /* storage blocked */ }
  },
}

export class ApiError extends Error {
  constructor(status, code, message) {
    super(message)
    this.status = status
    this.code = code
  }
}

let onUnauthorized = () => {}
export const setUnauthorizedHandler = (fn) => { onUnauthorized = fn }

export async function request(method, path, body, { token, raw } = {}) {
  const t = token === undefined ? tokenStore.get() : token
  const headers = {}
  if (t) headers.Authorization = `Bearer ${t}`
  if (body !== undefined) headers['Content-Type'] = 'application/json'
  let res
  try {
    res = await fetch(path, { method, headers, body: body !== undefined ? JSON.stringify(body) : undefined })
  } catch {
    throw new ApiError(0, 'network', 'Cannot reach the server. Check your connection.')
  }
  if (raw && res.ok) return res
  const text = await res.text()
  let data = null
  try { data = text ? JSON.parse(text) : null } catch { /* non-JSON body */ }
  if (!res.ok) {
    const e = data && data.error ? data.error : {}
    if (res.status === 401 && t && !path.startsWith('/v1/auth/login') && !path.startsWith('/v1/operator')) onUnauthorized()
    throw new ApiError(res.status, e.code || 'error', e.message || `Request failed (${res.status})`)
  }
  return data
}

export const api = {
  get: (p, o) => request('GET', p, undefined, o),
  post: (p, b, o) => request('POST', p, b ?? {}, o),
  patch: (p, b, o) => request('PATCH', p, b ?? {}, o),
  del: (p, o) => request('DELETE', p, undefined, o),
}

export const qs = (params) => {
  const u = new URLSearchParams()
  Object.entries(params).forEach(([k, v]) => { if (v !== undefined && v !== null && v !== '') u.set(k, v) })
  const s = u.toString()
  return s ? `?${s}` : ''
}
