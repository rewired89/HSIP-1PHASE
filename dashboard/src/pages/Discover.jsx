import React, { useState, useEffect } from 'react';
import { request } from '../api';

export default function Discover({ apiKey }) {
  const [agents,  setAgents]  = useState([]);
  const [loading, setLoading] = useState(true);
  const [registering, setRegistering] = useState(null);

  useEffect(() => { scan(); }, []);

  async function scan() {
    setLoading(true);
    try {
      const data = await request('GET', '/v1/agents/discover', null, apiKey);
      setAgents(Array.isArray(data) ? data : []);
    } catch {}
    setLoading(false);
  }

  async function register(agent) {
    setRegistering(agent.port);
    try {
      await request('POST', '/v1/keys', { name: agent.suggested_name, agent_type: 'ai_agent' }, apiKey);
      await scan();
    } catch (e) { alert(e.message); }
    setRegistering(null);
  }

  return (
    <div>
      <div className="card">
        <h2>Discover AI Agents</h2>
        <p style={{ color: '#718096', marginBottom: '1rem' }}>
          Probes well-known localhost ports (Ollama, LM Studio, dev servers, MCP-style
          agent ports) for anything currently running, so you can register a governance
          key for it in one click instead of hunting down its port yourself.
        </p>
        <button className="primary" onClick={scan} disabled={loading}>
          {loading ? 'Scanning…' : 'Scan again'}
        </button>
      </div>

      <div className="card">
        <h2>Found on this machine</h2>
        {loading && <p className="empty">Scanning localhost ports…</p>}
        {!loading && agents.length === 0 && (
          <p className="empty">Nothing detected on the known ports right now. Start an agent and scan again.</p>
        )}
        {!loading && agents.length > 0 && (
          <table>
            <thead>
              <tr><th>Port</th><th>Likely</th><th>Description</th><th>Status</th><th></th></tr>
            </thead>
            <tbody>
              {agents.map(a => (
                <tr key={a.port}>
                  <td><code>{a.port}</code></td>
                  <td>{a.hint}</td>
                  <td style={{ color: '#718096', fontSize: '0.85rem' }}>{a.description}</td>
                  <td>
                    <span className={`badge ${a.already_registered ? 'granted' : 'pending'}`}>
                      {a.already_registered ? 'registered' : 'not registered'}
                    </span>
                  </td>
                  <td>
                    {!a.already_registered && (
                      <button className="primary" onClick={() => register(a)} disabled={registering === a.port}>
                        {registering === a.port ? 'Registering…' : 'Register key'}
                      </button>
                    )}
                  </td>
                </tr>
              ))}
            </tbody>
          </table>
        )}
      </div>
    </div>
  );
}
