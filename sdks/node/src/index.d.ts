export interface ConsentRecord {
  id: string; peer_verify_key: string; status: string;
  granted_at?: number; expires_at?: number; revoked_at?: number; created_at: number;
}
export interface SignResponse   { id: string; content: string; signature: string; timestamp: number; }
export interface VerifyResponse { verified: boolean; peer_verify_key: string; timestamp: number; }
export interface IdentityResponse { tenant_id: string; verify_key: string; created_at: number; }
export interface AgentStats {
  key_id: string; name: string; active: boolean;
  request_count: number; anomaly_count: number; window_start_ms: number;
}
export interface CreateKeyResponse {
  id: string; key: string; name: string; agent_type: string;
  created_at: number; expires_at: number | null;
}
export interface DiscoveredAgent {
  port: number; url: string; hint: string; description: string;
  reachable: boolean; already_registered: boolean; suggested_name: string;
}

export class HSIPError extends Error { statusCode?: number; }

export class HSIPClient {
  constructor(opts: { apiKey: string; baseUrl?: string });

  // Identity
  getOrCreateIdentity(): Promise<IdentityResponse>;
  getIdentity(): Promise<IdentityResponse>;

  // Consent
  grantConsent(peerVerifyKey: string, ttlMs?: number): Promise<ConsentRecord>;
  revokeConsent(peerVerifyKey: string): Promise<ConsentRecord>;
  listConsents(): Promise<ConsentRecord[]>;
  getConsent(peerVerifyKey: string): Promise<ConsentRecord>;

  // Messages
  signMessage(content: string, peerVerifyKey?: string | null): Promise<SignResponse>;
  verifyMessage(content: string, signature: string, peerVerifyKey: string): Promise<VerifyResponse>;
  listMessages(): Promise<any[]>;

  // Audit
  getAuditLog(limit?: number, action?: string | null): Promise<any[]>;

  // API Keys
  createKey(name?: string, agentType?: string): Promise<CreateKeyResponse>;
  listKeys(): Promise<any[]>;
  revokeKey(keyId: string): Promise<any>;

  // AI Agent governance
  registerAgent(name: string, expiresDays?: number | null): Promise<CreateKeyResponse>;
  listAgents(): Promise<AgentStats[]>;
  revokeAgent(keyId: string): Promise<any>;
  logAction(action: string, detail?: string | null): Promise<SignResponse>;
  discoverAgents(): Promise<DiscoveredAgent[]>;
}
