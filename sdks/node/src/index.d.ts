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

export interface DecisionEnvelope {
  decision_id: string; tenant_id: string; agent_key_id: string;
  accountable_key: string; model_version: string; strategy_id: string;
  decision_type: string; payload_hash: string; prev_hash: string;
  timestamp_iso: string; timestamp_int: string; hsip_gov_ext: string;
}
export interface RecordDecisionResponse {
  decision_id: string; envelope: DecisionEnvelope; event_hash: string;
  signature: string; sign_algo: string; issuer_verify_key: string;
}
export interface DecisionSummary {
  id: string; decision_type: string; model_version: string; strategy_id: string;
  event_hash: string; prev_hash: string; timestamp_iso: string;
  anchored: boolean; anchor_id: string | null; merkle_index: number | null;
}
export interface ProofStep { hash: string; position: 'left' | 'right'; }
export interface DecisionProofBundle {
  envelope: DecisionEnvelope; event_hash: string; signature: string;
  sign_algo: string; issuer_verify_key: string; anchored: boolean;
  merkle_root: string | null; merkle_index: number | null;
  inclusion_proof: ProofStep[] | null;
  anchor_signature: string | null; anchor_verify_key: string | null;
  ots_status: string | null; ots_proof: string | null;
}
export interface VerifyDecisionBundle {
  envelope: DecisionEnvelope; event_hash: string; signature: string;
  issuer_verify_key: string;
  merkle_root?: string; inclusion_proof?: ProofStep[];
  anchor_signature?: string; anchor_verify_key?: string;
}
export interface VerifyDecisionResponse {
  valid: boolean; event_hash_matches: boolean; signature_valid: boolean;
  merkle_inclusion_valid: boolean | null; anchor_signature_valid: boolean | null;
  reason: string | null;
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

  // Decision attestations
  static hashPayload(payload: Buffer | string): string;
  recordDecision(opts: {
    accountableKey: string;
    modelVersion: string;
    strategyId: string;
    decisionType: string;
    payloadHash: string;
    receiptDir?: string;
  }): Promise<RecordDecisionResponse>;
  static saveReceipt(receipt: RecordDecisionResponse, receiptDir: string): string;
  listDecisions(): Promise<DecisionSummary[]>;
  getDecisionProof(decisionId: string): Promise<DecisionProofBundle>;
  verifyDecision(bundle: VerifyDecisionBundle): Promise<VerifyDecisionResponse>;
}
