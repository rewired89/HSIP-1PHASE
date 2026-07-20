// Package hsip provides a client for the HSIP REST API.
// Cryptographic consent and message verification for privacy-critical applications.
package hsip

import (
	"bytes"
	"crypto/sha256"
	"encoding/hex"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
	"os"
	"path/filepath"
)

// Client is the HSIP API client.
type Client struct {
	APIKey  string
	BaseURL string
	http    *http.Client
}

// New creates a new HSIP client.
func New(apiKey, baseURL string) *Client {
	return &Client{APIKey: apiKey, BaseURL: baseURL, http: &http.Client{}}
}

type APIError struct {
	StatusCode int
	Message    string
}

func (e *APIError) Error() string { return fmt.Sprintf("HSIP API %d: %s", e.StatusCode, e.Message) }

func (c *Client) do(method, path string, body, out any) error {
	var buf io.Reader
	if body != nil {
		b, err := json.Marshal(body)
		if err != nil {
			return err
		}
		buf = bytes.NewReader(b)
	}
	req, err := http.NewRequest(method, c.BaseURL+path, buf)
	if err != nil {
		return err
	}
	req.Header.Set("Authorization", "Bearer "+c.APIKey)
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.http.Do(req)
	if err != nil {
		return err
	}
	defer resp.Body.Close()

	respBody, _ := io.ReadAll(resp.Body)
	if resp.StatusCode >= 400 {
		var e struct {
			Error string `json:"error"`
		}
		json.Unmarshal(respBody, &e)
		return &APIError{StatusCode: resp.StatusCode, Message: e.Error}
	}
	return json.Unmarshal(respBody, out)
}

// Identity

type IdentityResponse struct {
	TenantID  string `json:"tenant_id"`
	VerifyKey string `json:"verify_key"`
	CreatedAt int64  `json:"created_at"`
}

func (c *Client) GetOrCreateIdentity() (*IdentityResponse, error) {
	var r IdentityResponse
	return &r, c.do("POST", "/v1/identity", nil, &r)
}
func (c *Client) GetIdentity() (*IdentityResponse, error) {
	var r IdentityResponse
	return &r, c.do("GET", "/v1/identity", nil, &r)
}

// Consent

type ConsentRecord struct {
	ID            string `json:"id"`
	PeerVerifyKey string `json:"peer_verify_key"`
	Status        string `json:"status"`
	GrantedAt     *int64 `json:"granted_at"`
	ExpiresAt     *int64 `json:"expires_at"`
	RevokedAt     *int64 `json:"revoked_at"`
	CreatedAt     int64  `json:"created_at"`
}

func (c *Client) GrantConsent(peerVerifyKey string, ttlMs int64) (*ConsentRecord, error) {
	var r ConsentRecord
	return &r, c.do("POST", "/v1/consent/grant", map[string]any{
		"peer_verify_key": peerVerifyKey, "ttl_ms": ttlMs,
	}, &r)
}
func (c *Client) RevokeConsent(peerVerifyKey string) (*ConsentRecord, error) {
	var r ConsentRecord
	return &r, c.do("POST", "/v1/consent/revoke", map[string]any{"peer_verify_key": peerVerifyKey}, &r)
}
func (c *Client) ListConsents() ([]ConsentRecord, error) {
	var r []ConsentRecord
	return r, c.do("GET", "/v1/consent", nil, &r)
}
func (c *Client) GetConsent(peerVerifyKey string) (*ConsentRecord, error) {
	var r ConsentRecord
	return &r, c.do("GET", "/v1/consent/"+peerVerifyKey, nil, &r)
}

// Messages

type SignResponse struct {
	ID        string `json:"id"`
	Content   string `json:"content"`
	Signature string `json:"signature"`
	Timestamp int64  `json:"timestamp"`
}
type VerifyResponse struct {
	Verified      bool   `json:"verified"`
	PeerVerifyKey string `json:"peer_verify_key"`
	Timestamp     int64  `json:"timestamp"`
}

func (c *Client) SignMessage(content, peerVerifyKey string) (*SignResponse, error) {
	body := map[string]any{"content": content}
	if peerVerifyKey != "" {
		body["peer_verify_key"] = peerVerifyKey
	}
	var r SignResponse
	return &r, c.do("POST", "/v1/messages/sign", body, &r)
}
func (c *Client) VerifyMessage(content, signature, peerVerifyKey string) (*VerifyResponse, error) {
	var r VerifyResponse
	return &r, c.do("POST", "/v1/messages/verify", map[string]any{
		"content": content, "signature": signature, "peer_verify_key": peerVerifyKey,
	}, &r)
}

// Messages (list)

type MessageRecord struct {
	ID        string `json:"id"`
	Content   string `json:"content"`
	Signature string `json:"signature"`
	Timestamp int64  `json:"timestamp"`
}

func (c *Client) ListMessages() ([]MessageRecord, error) {
	var r []MessageRecord
	return r, c.do("GET", "/v1/messages", nil, &r)
}

// Audit

type AuditEntry struct {
	ID            string  `json:"id"`
	Action        string  `json:"action"`
	PeerVerifyKey *string `json:"peer_verify_key"`
	Details       *string `json:"details"`
	Timestamp     int64   `json:"timestamp"`
}

func (c *Client) GetAuditLog(limit int) ([]AuditEntry, error) {
	var r []AuditEntry
	return r, c.do("GET", fmt.Sprintf("/v1/audit?limit=%d", limit), nil, &r)
}

// API Keys

type CreateKeyResponse struct {
	ID        string `json:"id"`
	Key       string `json:"key"`
	Name      string `json:"name"`
	AgentType string `json:"agent_type"`
	CreatedAt int64  `json:"created_at"`
	ExpiresAt *int64 `json:"expires_at"`
}

func (c *Client) CreateKey(name, agentType string) (*CreateKeyResponse, error) {
	var r CreateKeyResponse
	return &r, c.do("POST", "/v1/keys", map[string]any{"name": name, "agent_type": agentType}, &r)
}

func (c *Client) ListKeys() ([]map[string]any, error) {
	var r []map[string]any
	return r, c.do("GET", "/v1/keys", nil, &r)
}

func (c *Client) RevokeKey(keyID string) (map[string]any, error) {
	var r map[string]any
	return r, c.do("DELETE", "/v1/keys/"+keyID, nil, &r)
}

// AI Agent governance

type AgentStats struct {
	KeyID         string `json:"key_id"`
	Name          string `json:"name"`
	Active        bool   `json:"active"`
	RequestCount  uint64 `json:"request_count"`
	AnomalyCount  uint64 `json:"anomaly_count"`
	WindowStartMs int64  `json:"window_start_ms"`
}

type DiscoveredAgent struct {
	Port              uint16 `json:"port"`
	URL               string `json:"url"`
	Hint              string `json:"hint"`
	Description       string `json:"description"`
	Reachable         bool   `json:"reachable"`
	AlreadyRegistered bool   `json:"already_registered"`
	SuggestedName     string `json:"suggested_name"`
}

// RegisterAgent creates an AI agent key. expires_days=0 means no expiry.
func (c *Client) RegisterAgent(name string, expiresDays int) (*CreateKeyResponse, error) {
	body := map[string]any{"name": name, "agent_type": "ai_agent"}
	if expiresDays > 0 {
		body["expires_in_days"] = expiresDays
	}
	var r CreateKeyResponse
	return &r, c.do("POST", "/v1/keys", body, &r)
}

func (c *Client) ListAgents() ([]AgentStats, error) {
	var r []AgentStats
	return r, c.do("GET", "/v1/agents", nil, &r)
}

func (c *Client) RevokeAgent(keyID string) (map[string]any, error) {
	return c.RevokeKey(keyID)
}

// LogAction writes a signed, tamper-proof action record to the audit log.
// action is a short verb such as "file.read" or "email.send".
func (c *Client) LogAction(action, detail string) (*SignResponse, error) {
	content := "[ACTION:" + action + "]"
	if detail != "" {
		content += " " + detail
	}
	var r SignResponse
	return &r, c.do("POST", "/v1/messages/sign", map[string]any{"content": content}, &r)
}

func (c *Client) DiscoverAgents() ([]DiscoveredAgent, error) {
	var r []DiscoveredAgent
	return r, c.do("GET", "/v1/agents/discover", nil, &r)
}

// Decision attestations
//
// HSIP never receives or stores the actual content of a decision (trade
// parameters, etc.) — only its SHA-256 hash. Disclosure of the real
// payload, if ever needed, happens entirely on your side.

// DecisionEnvelope is the accountability metadata for one decision
// attestation — mirrors hsip-core::canonical::DecisionEnvelope.
type DecisionEnvelope struct {
	DecisionID     string `json:"decision_id"`
	TenantID       string `json:"tenant_id"`
	AgentKeyID     string `json:"agent_key_id"`
	AccountableKey string `json:"accountable_key"`
	ModelVersion   string `json:"model_version"`
	StrategyID     string `json:"strategy_id"`
	DecisionType   string `json:"decision_type"`
	PayloadHash    string `json:"payload_hash"`
	PrevHash       string `json:"prev_hash"`
	TimestampISO   string `json:"timestamp_iso"`
	TimestampInt   string `json:"timestamp_int"`
	HsipGovExt     string `json:"hsip_gov_ext"`
}

// RecordDecisionResponse is the self-contained receipt returned by
// RecordDecision — keep it, it's the client-side mitigation for the gap
// between signing and anchoring.
type RecordDecisionResponse struct {
	DecisionID      string           `json:"decision_id"`
	Envelope        DecisionEnvelope `json:"envelope"`
	EventHash       string           `json:"event_hash"`
	Signature       string           `json:"signature"`
	SignAlgo        string           `json:"sign_algo"`
	IssuerVerifyKey string           `json:"issuer_verify_key"`
}

type DecisionSummary struct {
	ID           string  `json:"id"`
	DecisionType string  `json:"decision_type"`
	ModelVersion string  `json:"model_version"`
	StrategyID   string  `json:"strategy_id"`
	EventHash    string  `json:"event_hash"`
	PrevHash     string  `json:"prev_hash"`
	TimestampISO string  `json:"timestamp_iso"`
	Anchored     bool    `json:"anchored"`
	AnchorID     *string `json:"anchor_id"`
	MerkleIndex  *int64  `json:"merkle_index"`
}

// ProofStep is one RFC 6962 Merkle inclusion-proof step.
type ProofStep struct {
	Hash     string `json:"hash"`
	Position string `json:"position"` // "left" | "right"
}

// DecisionProofBundle is the full self-contained verification bundle for
// one decision — signature always present; Merkle/anchor fields are nil
// until the next anchor cycle picks this decision up.
type DecisionProofBundle struct {
	Envelope        DecisionEnvelope `json:"envelope"`
	EventHash       string           `json:"event_hash"`
	Signature       string           `json:"signature"`
	SignAlgo        string           `json:"sign_algo"`
	IssuerVerifyKey string           `json:"issuer_verify_key"`
	Anchored        bool             `json:"anchored"`
	MerkleRoot      *string          `json:"merkle_root"`
	MerkleIndex     *int64           `json:"merkle_index"`
	InclusionProof  []ProofStep      `json:"inclusion_proof"`
	AnchorSignature *string          `json:"anchor_signature"`
	AnchorVerifyKey *string          `json:"anchor_verify_key"`
	OtsStatus       *string          `json:"ots_status"`
	OtsProof        *string          `json:"ots_proof"`
}

// VerifyDecisionRequest is the bundle passed to VerifyDecision — same
// shape as DecisionProofBundle's provable fields, submitted back to
// POST /v1/decisions/verify (a pure function of this body: no API key,
// no database — any third party can run the equivalent check themselves).
type VerifyDecisionRequest struct {
	Envelope        DecisionEnvelope `json:"envelope"`
	EventHash       string           `json:"event_hash"`
	Signature       string           `json:"signature"`
	IssuerVerifyKey string           `json:"issuer_verify_key"`
	MerkleRoot      *string          `json:"merkle_root,omitempty"`
	InclusionProof  []ProofStep      `json:"inclusion_proof,omitempty"`
	AnchorSignature *string          `json:"anchor_signature,omitempty"`
	AnchorVerifyKey *string          `json:"anchor_verify_key,omitempty"`
}

type VerifyDecisionResponse struct {
	Valid                bool    `json:"valid"`
	EventHashMatches     bool    `json:"event_hash_matches"`
	SignatureValid       bool    `json:"signature_valid"`
	MerkleInclusionValid *bool   `json:"merkle_inclusion_valid"`
	AnchorSignatureValid *bool   `json:"anchor_signature_valid"`
	Reason               *string `json:"reason"`
}

// HashPayload returns the hex-encoded SHA-256 of a decision payload,
// ready for RecordDecisionOpts.PayloadHash. Not a Client method — it
// needs no server state, matching the SDK's other language bindings.
func HashPayload(payload []byte) string {
	sum := sha256.Sum256(payload)
	return hex.EncodeToString(sum[:])
}

// SaveReceipt persists a decision receipt to
// <receiptDir>/<decision_id>.json and returns the path written. Safe to
// call independently of RecordDecision (e.g. to re-save a receipt fetched
// later).
func SaveReceipt(receipt *RecordDecisionResponse, receiptDir string) (string, error) {
	if err := os.MkdirAll(receiptDir, 0o755); err != nil {
		return "", err
	}
	data, err := json.MarshalIndent(receipt, "", "  ")
	if err != nil {
		return "", err
	}
	path := filepath.Join(receiptDir, receipt.DecisionID+".json")
	if err := os.WriteFile(path, data, 0o644); err != nil {
		return "", err
	}
	return path, nil
}

// RecordDecisionOpts are the fields needed to sign and chain one
// AI-agent decision attestation. PayloadHash must be the hex-encoded
// SHA-256 of your actual (never disclosed to HSIP) decision content —
// see HashPayload. ReceiptDir, if non-empty, also writes the receipt to
// disk immediately via SaveReceipt.
type RecordDecisionOpts struct {
	AccountableKey string
	ModelVersion   string
	StrategyID     string
	DecisionType   string
	PayloadHash    string
	ReceiptDir     string
}

// RecordDecision signs and chains one AI-agent decision attestation,
// returning a self-contained receipt.
func (c *Client) RecordDecision(opts RecordDecisionOpts) (*RecordDecisionResponse, error) {
	var r RecordDecisionResponse
	if err := c.do("POST", "/v1/decisions", map[string]any{
		"accountable_key": opts.AccountableKey,
		"model_version":   opts.ModelVersion,
		"strategy_id":     opts.StrategyID,
		"decision_type":   opts.DecisionType,
		"payload_hash":    opts.PayloadHash,
	}, &r); err != nil {
		return nil, err
	}
	if opts.ReceiptDir != "" {
		if _, err := SaveReceipt(&r, opts.ReceiptDir); err != nil {
			return &r, err
		}
	}
	return &r, nil
}

// ListDecisions lists this tenant's decision attestations, newest first.
func (c *Client) ListDecisions() ([]DecisionSummary, error) {
	var r []DecisionSummary
	return r, c.do("GET", "/v1/decisions", nil, &r)
}

// GetDecisionProof returns the full self-contained verification bundle
// for one decision. Before the next anchor cycle runs, Anchored is false
// and only authorship (signature) is provable yet — call again later
// once a batch anchors.
func (c *Client) GetDecisionProof(decisionID string) (*DecisionProofBundle, error) {
	var r DecisionProofBundle
	return &r, c.do("GET", "/v1/decisions/"+decisionID+"/proof", nil, &r)
}

// VerifyDecision verifies a decision proof bundle. This calls HSIP's
// /v1/decisions/verify endpoint, but that endpoint takes no API key and
// touches no database — it's a pure function of bundle, so any party can
// run the equivalent check themselves without this SDK or an HSIP
// account at all.
func (c *Client) VerifyDecision(bundle *VerifyDecisionRequest) (*VerifyDecisionResponse, error) {
	var r VerifyDecisionResponse
	return &r, c.do("POST", "/v1/decisions/verify", bundle, &r)
}
