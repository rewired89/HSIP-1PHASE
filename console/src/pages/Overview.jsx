import { useState } from 'react'
import { Link, useNavigate } from 'react-router-dom'
import { useOrg } from '../auth.jsx'
import { ago, fmtNum, short } from '../format.js'
import { AnchorBadge, BarChart, Card, Empty, Gate, Meter, PageHeader, Stat, useLoad } from '../ui.jsx'

function Checklist({ data, streams, keys }) {
  const steps = [
    ['Create a stream', 'A stream is one hash chain, usually one system or workflow.', streams > 0, '/streams'],
    ['Create an API key', 'Scoped to ingest, optionally bound to one stream.', keys > 0, '/api-keys'],
    ['Record your first proof', 'Use the quickstart snippet or send a test proof from a stream.', data.total_proofs > 0, '/quickstart'],
    ['Export evidence', 'Generate a signed pack and verify it offline.', false, '/evidence'],
  ]
  return (
    <Card title="Get started">
      <div className="col">
        {steps.map(([t, d, done, to], i) => (
          <div className="row" key={t}>
            <span className={`badge ${done ? 'ok' : ''}`} style={{ width: 26, justifyContent: 'center' }}>{done ? '✓' : i + 1}</span>
            <div className="grow"><strong>{t}</strong><div className="small muted">{d}</div></div>
            {!done && <Link className="btn sm" to={to}>Open</Link>}
          </div>
        ))}
      </div>
    </Card>
  )
}

export default function Overview() {
  const org = useOrg()
  const nav = useNavigate()
  const [range, setRange] = useState('24h')
  const st = useLoad(async () => {
    const [ov, streams, keys] = await Promise.all([
      org.get('/overview'),
      org.get('/streams'),
      org.isAdmin ? org.get('/api-keys') : Promise.resolve({ keys: [{}] }),
    ])
    return { ov, streams: streams.streams.filter((s) => s.kind === 'user').length, keys: keys.keys.length }
  }, [org.id])

  return (
    <>
      <PageHeader title="Overview" sub="Activity across your proof streams">
        <Link className="btn" to="/verify">Verify a proof</Link>
        <Link className="btn primary" to="/evidence">Export evidence</Link>
      </PageHeader>
      <Gate state={st}>
        {({ ov, streams, keys }) => {
          const u = ov.usage
          const pctAnchored = ov.total_proofs ? Math.round((ov.anchored_proofs / ov.total_proofs) * 100) : 0
          const series = range === '24h' ? ov.hourly : ov.daily
          const step = range === '24h' ? 3_600_000 : 86_400_000
          const labels = series.counts.map((_, i) => {
            const d = new Date(series.start_ms + i * step)
            return range === '24h' ? d.toLocaleTimeString([], { hour: '2-digit', minute: '2-digit' }) : d.toLocaleDateString([], { month: 'short', day: 'numeric' })
          })
          return (
            <div className="col" style={{ gap: 16 }}>
              {ov.total_proofs === 0 && <Checklist data={ov} streams={streams} keys={keys} />}
              <div className="grid g4">
                <Stat label="Proofs this month" value={fmtNum(u.proofs_used)} detail={`of ${fmtNum(u.plan.proofs_per_month)} on ${u.plan.name}`} />
                <Stat label="Total proofs" value={fmtNum(ov.total_proofs)} detail={`${streams} active stream${streams === 1 ? '' : 's'}`} />
                <Stat label="Anchored" value={`${pctAnchored}%`} detail={`${fmtNum(ov.anchored_proofs)} in signed batches`} />
                <Stat label="Awaiting anchor" value={fmtNum(ov.pending_anchor)} detail="batched every minute or so" />
              </div>
              <div className="grid g-main">
                <Card
                  title="Proofs recorded"
                  actions={
                    <div className="row">
                      <button className={`btn sm ${range === '24h' ? 'primary' : ''}`} onClick={() => setRange('24h')}>24 hours</button>
                      <button className={`btn sm ${range === '30d' ? 'primary' : ''}`} onClick={() => setRange('30d')}>30 days</button>
                    </div>
                  }
                >
                  <BarChart counts={series.counts} labels={labels} />
                </Card>
                <Card title="Plan usage">
                  <div className="col">
                    <Meter used={u.proofs_used} limit={u.plan.proofs_per_month} />
                    <div className="small muted">Streams {u.streams_used}/{fmtNum(u.plan.streams)} · Seats {u.seats_used}/{fmtNum(u.plan.seats)}</div>
                    <Link to="/billing" className="btn sm">Plan & usage</Link>
                  </div>
                </Card>
              </div>
              <div className="grid g2">
                <Card title="Top actions (30 days)">
                  {ov.top_actions.length === 0 ? <div className="muted">Nothing yet.</div> : (
                    <div className="col">{ov.top_actions.map((a) => <div className="row" key={a.name}><code className="grow truncate">{a.name}</code><strong>{fmtNum(a.count)}</strong></div>)}</div>
                  )}
                </Card>
                <Card title="Top actors (30 days)">
                  {ov.top_actors.length === 0 ? <div className="muted">Nothing yet.</div> : (
                    <div className="col">{ov.top_actors.map((a) => <div className="row" key={a.name}><span className="grow truncate">{a.name}</span><strong>{fmtNum(a.count)}</strong></div>)}</div>
                  )}
                </Card>
              </div>
              <Card title="Latest records" flush actions={<Link to="/records" className="btn sm">All records</Link>}>
                {ov.recent.length === 0 ? <Empty title="No records yet">Send your first proof from the quickstart.</Empty> : (
                  <div className="table-wrap">
                    <table className="t">
                      <thead><tr><th>When</th><th>Actor</th><th>Action</th><th>Subject</th><th>Hash</th><th>Status</th></tr></thead>
                      <tbody>
                        {ov.recent.map((p) => (
                          <tr key={p.id} className="click" onClick={() => nav(`/records/${p.id}`)}>
                            <td className="nowrap">{ago(p.created_at)}</td>
                            <td>{p.actor}</td>
                            <td><code>{p.action}</code></td>
                            <td className="truncate" style={{ maxWidth: 200 }}>{p.subject || '–'}</td>
                            <td><span className="hash">{short(p.event_hash, 8, 4)}</span></td>
                            <td><AnchorBadge anchored={p.anchored} /></td>
                          </tr>
                        ))}
                      </tbody>
                    </table>
                  </div>
                )}
              </Card>
            </div>
          )
        }}
      </Gate>
    </>
  )
}
