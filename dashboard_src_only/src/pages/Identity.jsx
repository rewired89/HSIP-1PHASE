import React, { useState } from "react";
import { request } from "../api";
export default function Identity({ apiKey }) {
  const [identity, setIdentity] = useState(null);
  const [loading,  setLoading]  = useState(false);
  async function load() {
    setLoading(true);
    try { setIdentity(await request("POST", "/v1/identity", null, apiKey)); }
    catch (e) { alert(e.message); }
    finally { setLoading(false); }
  }
  return (
    <div>
      <div className="card">
        <h2>HSIP Identity</h2>
        <p style={{color: "#718096", marginBottom: "1rem"}}>
          Your Ed25519 cryptographic identity. Share your verify key so peers can verify your signatures.
        </p>
        <button className="primary" onClick={load} disabled={loading}>
          {loading ? "Loading..." : "Get / Create Identity"}
        </button>
      </div>
      {identity && (
        <div className="card">
          <h2>Your Public Key (Verify Key)</h2>
          <div className="key-display">{identity.verify_key}</div>
          <p style={{color: "#718096", marginTop: "0.75rem", fontSize: "0.8rem"}}>
            Tenant ID: {identity.tenant_id} | Created: {new Date(identity.created_at).toLocaleString()}
          </p>
        </div>
      )}
    </div>
  );
}