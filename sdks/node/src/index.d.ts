export interface ConsentRecord {
  id: string; peer_verify_key: string; status: string;
  granted_at?: number; expires_at?: number; revoked_at?: number; created_at: number;
}
export interface SignResponse   { id: string; content: string; signature: string; timestamp: number; }
export interface VerifyResponse { verified: boolean; peer_verify_key: string; timestamp: number; }
export interface IdentityResponse { tenant_id: string; verify_key: string; created_at: number; }

export class HSIPError extends Error { statusCode?: number; }

export class HSIPClient {
  constructor(opts: { apiKey: string; baseUrl?: string });
  getOrCreateIdentity(): Promise<IdentityResponse>;
  getIdentity(): Promise<IdentityResponse>;
  grantConsent(peerVerifyKey: string, ttlMs?: number): Promise<ConsentRecord>;
  revokeConsent(peerVerifyKey: string): Promise<ConsentRecord>;
  listConsents(): Promise<ConsentRecord[]>;
  getConsent(peerVerifyKey: string): Promise<ConsentRecord>;
  signMessage(content: string, peerVerifyKey?: string): Promise<SignResponse>;
  verifyMessage(content: string, signature: string, peerVerifyKey: string): Promise<VerifyResponse>;
  listMessages(): Promise<any[]>;
  getAuditLog(limit?: number, action?: string): Promise<any[]>;
  createKey(name?: string): Promise<any>;
  listKeys(): Promise<any[]>;
  revokeKey(keyId: string): Promise<any>;
}
