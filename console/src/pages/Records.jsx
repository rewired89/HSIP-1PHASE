import { useEffect, useState } from 'react'
import { useNavigate } from 'react-router-dom'
import { qs } from '../api.js'
import { useOrg } from '../auth.jsx'
import { ago, fmtDate, short, toInputDate } from '../format.js'
import { AnchorBadge, Card, Empty, Gate, PageHeader, useLoad } from '../ui.jsx'

const PAGE = 25

/** Filterable, paged proof table. Pass `streamId` to pin it to one stream. */
export function RecordsTable({ streamId, showStreamFilter, reloadKey }) {
  const org = useOrg()
  const nav = useNavigate()
  const [f, setF] = useState({ q: '', stream_id: streamId || '', anchored: '', from: '', to: '' })
  const [q, setQ] = useState('')
  const [page, setPage] = useState(0)
  useEffect(() => { const t = setTimeout(() => { setQ(f.q); setPage(0) }, 300); return () => clearTimeout(t) }, [f.q])

  const streams = useLoad(() => (showStreamFilter ? org.get('/streams') : Promise.resolve({ streams: [] })), [org.id])
  const st = useLoad(
    () => org.get('/proofs' + qs({
      stream_id: streamId || f.stream_id,
      q,
      anchored: f.anchored,
      from_ms: f.from ? new Date(f.from + 'T00:00:00').getTime() : '',
      to_ms: f.to ? new Date(f.to + 'T23:59:59').getTime() : '',
      limit: PAGE,
      offset: page * PAGE,
    })),
    [org.id, streamId, f.stream_id, q, f.anchored, f.from, f.to, page, reloadKey],
  )

  const set = (k) => (e) => { setF({ ...f, [k]: e.target.value }); setPage(0) }
  return (
    <Card flush>
      <div className="row wrap" style={{ padding: 14, borderBottom: '1px solid var(--line)' }}>
        <input style={{ maxWidth: 260 }} placeholder="Search actor, action, subject, hash…" value={f.q} onChange={(e) => setF({ ...f, q: e.target.value })} aria-label="Search" />
        {showStreamFilter && (
          <select className="inline-input" value={f.stream_id} onChange={set('stream_id')} aria-label="Stream">
            <option value="">All streams</option>
            {(streams.data?.streams || []).filter((s) => s.kind === 'user').map((s) => <option key={s.id} value={s.id}>{s.name}</option>)}
          </select>
        )}
        <select className="inline-input" value={f.anchored} onChange={set('anchored')} aria-label="Anchor status">
          <option value="">Any status</option>
          <option value="true">Anchored</option>
          <option value="false">Awaiting anchor</option>
        </select>
        <input className="inline-input" type="date" max={toInputDate(Date.now())} value={f.from} onChange={set('from')} aria-label="From date" />
        <span className="muted">to</span>
        <input className="inline-input" type="date" max={toInputDate(Date.now())} value={f.to} onChange={set('to')} aria-label="To date" />
      </div>
      <Gate state={st}>
        {(d) => d.items.length === 0 ? (
          <Empty title="No records match">Records appear here the moment your systems send them.</Empty>
        ) : (
          <>
            <div className="table-wrap">
              <table className="t">
                <thead><tr><th>#</th><th>When</th><th>Actor</th><th>Action</th><th>Subject</th><th>Payload hash</th><th>Status</th></tr></thead>
                <tbody>
                  {d.items.map((p) => (
                    <tr key={p.id} className="click" onClick={() => nav(`/records/${p.id}`)}>
                      <td className="muted">{p.seq}</td>
                      <td className="nowrap" title={fmtDate(p.created_at)}>{ago(p.created_at)}</td>
                      <td>{p.actor}</td>
                      <td><code>{p.action}</code></td>
                      <td className="truncate" style={{ maxWidth: 200 }}>{p.subject || '–'}</td>
                      <td><span className="hash">{short(p.payload_hash, 8, 4)}</span></td>
                      <td><AnchorBadge anchored={p.anchored} /></td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
            <div className="row" style={{ padding: 12, borderTop: '1px solid var(--line)' }}>
              <span className="muted small grow">{d.total.toLocaleString()} record{d.total === 1 ? '' : 's'} · page {page + 1} of {Math.max(1, Math.ceil(d.total / PAGE))}</span>
              <button className="btn sm" disabled={page === 0} onClick={() => setPage(page - 1)}>Previous</button>
              <button className="btn sm" disabled={(page + 1) * PAGE >= d.total} onClick={() => setPage(page + 1)}>Next</button>
            </div>
          </>
        )}
      </Gate>
    </Card>
  )
}

export default function Records() {
  return (
    <>
      <PageHeader title="Records" sub="Every proof recorded across your streams" />
      <RecordsTable showStreamFilter />
    </>
  )
}
