// Package hsip provides a client for the HSIP REST API.
// Cryptographic consent and message verification for privacy-critical applications.
package hsip

import (
	"bytes"
	"encoding/json"
	"fmt"
	"io"
	"net/http"
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
		if err != nil { return err }
		buf = bytes.NewReader(b)
	}
	req, err := http.NewRequest(method, c.BaseURL+path, buf)
	if err != nil { return err }
	req.Header.Set("Authorization", "Bearer "+c.APIKey)
	req.Header.Set("Content-Type", "application/json")

	resp, err := c.http.Do(req)
	if err != nil { return err }
	defer resp.Body.Close()

	respBody, _ := io.ReadAll(resp.Body)
	if resp.StatusCode >= 400 {
		var e struct{ Error string `json:"error"` }
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
	ID             string  `json:"id"`
	PeerVerifyKey  string  `json:"peer_verify_key"`
	Status         string  `json:"status"`
	GrantedAt      *int64  `json:"granted_at"`
	ExpiresAt      *int64  `json:"expires_at"`
	RevokedAt      *int64  `json:"revoked_at"`
	CreatedAt      int64   `json:"created_at"`
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
	if peerVerifyKey != "" { body["peer_verify_key"] = peerVerifyKey }
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
	ID        string  `json:"id"`
	Key       string  `json:"key"`
	Name      string  `json:"name"`
	AgentType string  `json:"agent_type"`
	CreatedAt int64   `json:"created_at"`
	ExpiresAt *int64  `json:"expires_at"`
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
	KeyID        string `json:"key_id"`
	Name         string `json:"name"`
	Active       bool   `json:"active"`
	RequestCount uint64 `json:"request_count"`
	AnomalyCount uint64 `json:"anomaly_count"`
	WindowStartMs int64 `json:"window_start_ms"`
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
