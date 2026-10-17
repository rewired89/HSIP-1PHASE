import { useOrg } from '../auth.jsx'
import { Empty, Gate, PageHeader, useLoad } from '../ui.jsx'
import { RecordsTable } from './Records.jsx'

export default function Activity() {
  const org = useOrg()
  const st = useLoad(() => org.get('/streams'), [org.id])
  return (
    <>
      <PageHeader title="Audit trail" sub="Our own actions on your account (sign-ins, key and team changes, exports) are recorded as signed, chained proofs in your system stream, using the same machinery you are buying." />
      <Gate state={st}>
        {(d) => {
          const sys = d.streams.find((s) => s.kind === 'system')
          return sys ? <RecordsTable streamId={sys.id} /> : <Empty title="No audit stream" />
        }}
      </Gate>
    </>
  )
}
