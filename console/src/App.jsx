import { BrowserRouter, Navigate, Outlet, Route, Routes } from 'react-router-dom'
import { AuthProvider, useAuth } from './auth.jsx'
import { Loading, ToastProvider } from './ui.jsx'
import Shell from './Shell.jsx'
import Landing from './pages/Landing.jsx'
import { Invite, Login, Signup } from './pages/AuthPages.jsx'
import Overview from './pages/Overview.jsx'
import Streams from './pages/Streams.jsx'
import StreamDetail from './pages/StreamDetail.jsx'
import Records from './pages/Records.jsx'
import ProofDetail from './pages/ProofDetail.jsx'
import Evidence from './pages/Evidence.jsx'
import Verify from './pages/Verify.jsx'
import SharedProof from './pages/SharedProof.jsx'
import ApiKeys from './pages/ApiKeys.jsx'
import Quickstart from './pages/Quickstart.jsx'
import Team from './pages/Team.jsx'
import Billing from './pages/Billing.jsx'
import Activity from './pages/Activity.jsx'
import Settings from './pages/Settings.jsx'
import Operator from './pages/Operator.jsx'
import { PublicFrame } from './pages/Landing.jsx'

function Protected() {
  const { status } = useAuth()
  if (status === 'loading') return <Loading />
  if (status === 'out') return <Navigate to="/login" replace />
  return <Shell><Outlet /></Shell>
}

function Home() {
  const { status } = useAuth()
  if (status === 'loading') return <Loading />
  if (status === 'out') return <Landing />
  return <Shell><Overview /></Shell>
}

/** Pages that work signed in (inside the shell) or signed out (standalone). */
function Either({ children }) {
  const { status } = useAuth()
  if (status === 'loading') return <Loading />
  if (status === 'in') return <Shell>{children}</Shell>
  return <PublicFrame>{children}</PublicFrame>
}

export default function App() {
  return (
    <BrowserRouter>
      <ToastProvider>
        <AuthProvider>
          <Routes>
            <Route path="/" element={<Home />} />
            <Route path="/login" element={<Login />} />
            <Route path="/signup" element={<Signup />} />
            <Route path="/invite/:token" element={<Invite />} />
            <Route path="/verify" element={<Either><Verify /></Either>} />
            <Route path="/p/:token" element={<PublicFrame><SharedProof /></PublicFrame>} />
            <Route path="/operator" element={<PublicFrame><Operator /></PublicFrame>} />
            <Route element={<Protected />}>
              <Route path="/streams" element={<Streams />} />
              <Route path="/streams/:sid" element={<StreamDetail />} />
              <Route path="/records" element={<Records />} />
              <Route path="/records/:pid" element={<ProofDetail />} />
              <Route path="/evidence" element={<Evidence />} />
              <Route path="/activity" element={<Activity />} />
              <Route path="/quickstart" element={<Quickstart />} />
              <Route path="/api-keys" element={<ApiKeys />} />
              <Route path="/team" element={<Team />} />
              <Route path="/billing" element={<Billing />} />
              <Route path="/settings" element={<Settings />} />
            </Route>
            <Route path="*" element={<Navigate to="/" replace />} />
          </Routes>
        </AuthProvider>
      </ToastProvider>
    </BrowserRouter>
  )
}
