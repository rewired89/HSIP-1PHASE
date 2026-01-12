# HSIP Phase 2 - API Authentication & Authorization

Complete JWT-based authentication and role-based access control (RBAC) for enterprise banking security.

## Overview

The HSIP API uses industry-standard JWT (JSON Web Tokens) for stateless authentication with:
- **Role-Based Access Control (RBAC)**: Admin, BankAdmin, ApiUser, Auditor, EndUser
- **Fine-Grained Scopes**: message:send, message:verify, consent:grant, audit:export
- **Audit Logging**: All authentication events logged to PostgreSQL
- **Secure Defaults**: HS256 signing, token expiration, issuer/audience validation

## Authentication Flow

### 1. Login (Get JWT Token)

**Endpoint:** `POST /api/v2/auth/login`

**Request:**
```json
{
  "username": "bank_admin",
  "password": "secure_password"
}
```

**Response:**
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "token_type": "Bearer",
  "expires_in": 86400,
  "roles": ["bank_admin"],
  "scopes": ["message:send", "message:verify"]
}
```

**Using the token:**
```bash
curl -H "Authorization: Bearer eyJhbGciOiJIUzI1NiIsInR5..." \
  http://localhost:8787/api/v2/messages/send
```

### 2. API Key Creation (Admin Only)

**Endpoint:** `POST /api/v2/auth/apikey`

**Headers:**
```
Authorization: Bearer <admin_token>
```

**Request:**
```json
{
  "name": "Production API Key - Chase Bank",
  "description": "Key for automated transaction alerts",
  "roles": ["api_user"],
  "scopes": ["message:send", "message:verify"],
  "expires_in_days": 365
}
```

**Response:**
```json
{
  "api_key": "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9...",
  "key_id": "550e8400-e29b-41d4-a716-446655440000",
  "expires_at": "2025-12-15T14:30:00Z"
}
```

### 3. Token Info (Validate Token)

**Endpoint:** `GET /api/v2/auth/token/info`

**Headers:**
```
Authorization: Bearer <token>
```

**Response:**
```json
{
  "subject": "bank_admin",
  "issuer": "hsip-api",
  "audience": "hsip-clients",
  "issued_at": 1734278400,
  "expires_at": 1734364800,
  "roles": ["bank_admin"],
  "scopes": ["message:send", "message:verify"]
}
```

### 4. Refresh Token

**Endpoint:** `POST /api/v2/auth/token/refresh`

**Headers:**
```
Authorization: Bearer <current_token>
```

**Response:**
```json
{
  "access_token": "eyJhbGciOiJIUzI1NiIsInR5...",
  "token_type": "Bearer",
  "expires_in": 86400,
  "roles": ["bank_admin"],
  "scopes": ["message:send", "message:verify"]
}
```

## Roles & Permissions

### Role Hierarchy

| Role | Description | Permissions |
|------|-------------|-------------|
| `admin` | System administrator | **ALL** (full access to everything) |
| `bank_admin` | Bank administrator | Send/verify messages for their bank |
| `api_user` | API integration user | Send and verify messages |
| `auditor` | Compliance auditor | Read-only access to audit logs |
| `end_user` | End user (consumer) | Manage own consent policies |

### Scopes (Fine-Grained Permissions)

| Scope | Description | Endpoints |
|-------|-------------|-----------|
| `message:send` | Send verified messages | `POST /api/v2/messages/send` |
| `message:verify` | Verify message signatures | `POST /api/v2/messages/verify` |
| `consent:grant` | Grant consent policies | `POST /api/v2/consent/grant` |
| `consent:revoke` | Revoke consent policies | `POST /api/v2/consent/revoke` |
| `audit:export` | Export audit logs | `POST /api/v2/audit/export` |
| `audit:read` | Read audit logs | `GET /api/v2/audit/logs` |

### Permission Matrix

| Role | message:send | message:verify | consent:grant | consent:revoke | audit:export | audit:read |
|------|--------------|----------------|---------------|----------------|--------------|------------|
| admin | ✅ | ✅ | ✅ | ✅ | ✅ | ✅ |
| bank_admin | ✅ | ✅ | ✅ | ✅ | ❌ | ❌ |
| api_user | ✅ | ✅ | ❌ | ❌ | ❌ | ❌ |
| auditor | ❌ | ❌ | ❌ | ❌ | ✅ | ✅ |
| end_user | ❌ | ❌ | ✅ | ✅ | ❌ | ❌ |

## Configuration

### Environment Variables

```bash
# JWT secret (CRITICAL: change in production!)
export JWT_SECRET="your-256-bit-secret-key-change-this"

# Issuer (identifies who issued the token)
export JWT_ISSUER="hsip-api"

# Audience (identifies who the token is for)
export JWT_AUDIENCE="hsip-clients"

# Database connection (for audit logging)
export DATABASE_URL="postgres://hsip:password@localhost:5432/hsip_prod"
```

**Generate secure secret:**
```bash
# 256-bit secret (32 bytes)
openssl rand -base64 32
```

### Rust Integration

```rust
use hsip_api::{JwtManager, Role, Scope};

// Create JWT manager
let jwt_manager = JwtManager::from_env();

// Generate token
let token = jwt_manager.generate_token(
    "user123",                           // subject
    vec!["api_user".to_string()],       // roles
    vec!["message:send".to_string()],   // scopes
    24,                                  // expires in 24 hours
)?;

// Validate token
let claims = jwt_manager.validate_token(&token)?;

// Check role
if jwt_manager.has_role(&claims, &Role::ApiUser) {
    // User has api_user role
}

// Check scope
if jwt_manager.has_scope(&claims, &Scope::MessageSend) {
    // User can send messages
}
```

## API Endpoint Protection

### Protecting Endpoints with Middleware

```rust
use axum::{
    routing::{get, post},
    Router,
    middleware,
};
use hsip_api::auth::{auth_middleware, require_role, require_scope, Role, Scope};

let app = Router::new()
    // Public endpoints (no auth required)
    .route("/health", get(health_check))

    // Protected endpoints (requires authentication)
    .route("/api/v2/messages/send", post(send_message))
    .route("/api/v2/messages/verify", post(verify_message))

    // Admin-only endpoints
    .route("/api/v2/admin/users", get(list_users))
        .layer(middleware::from_fn(require_role(Role::Admin)))

    // Add auth middleware globally
    .layer(middleware::from_fn(auth_middleware));
```

### Handler with AuthContext

```rust
use axum::Json;
use hsip_api::auth::AuthContext;

async fn send_message(
    auth: AuthContext,  // Automatically extracts and validates JWT
    Json(req): Json<SendMessageRequest>,
) -> ApiResult<Json<SendMessageResponse>> {
    // Auth context contains validated claims
    tracing::info!("User {} sending message", auth.claims.sub);

    // Check specific permission
    if !jwt_manager.has_scope(&auth.claims, &Scope::MessageSend) {
        return Err(ApiError::Unauthorized("Insufficient permissions".to_string()));
    }

    // Process request...
}
```

## Security Best Practices

### 1. Token Expiration

**Default:** 24 hours
**API Keys:** 365 days (long-lived)

**Refresh tokens before expiry:**
```bash
# Check expiration
jwt decode $TOKEN | jq .exp

# Refresh before expiry
curl -X POST http://localhost:8787/api/v2/auth/token/refresh \
  -H "Authorization: Bearer $TOKEN"
```

### 2. Secret Key Management

**❌ DO NOT:**
- Commit secrets to Git
- Use default secrets in production
- Share secrets between environments

**✅ DO:**
- Use environment variables
- Rotate secrets periodically
- Use different secrets per environment

**Example with Docker Secrets:**
```yaml
services:
  hsip-api:
    environment:
      JWT_SECRET: ${JWT_SECRET}
    secrets:
      - jwt_secret

secrets:
  jwt_secret:
    external: true
```

### 3. HTTPS Only

**Always use HTTPS in production:**
```nginx
server {
    listen 443 ssl;
    server_name api.hsip.io;

    ssl_certificate /path/to/cert.pem;
    ssl_certificate_key /path/to/key.pem;

    location /api {
        proxy_pass http://localhost:8787;
        proxy_set_header Authorization $http_authorization;
    }
}
```

### 4. Rate Limiting

Prevent brute-force attacks:
```rust
use tower::limit::RateLimitLayer;

let app = Router::new()
    .route("/api/v2/auth/login", post(login))
    .layer(RateLimitLayer::new(5, Duration::from_secs(60))); // 5 requests per minute
```

### 5. Audit Logging

All authentication events are logged:
```sql
SELECT * FROM audit_log
WHERE event_type IN ('user_login', 'api_key_created', 'auth_failed')
ORDER BY timestamp DESC
LIMIT 100;
```

## Testing

### Unit Tests

```bash
# Run auth module tests
cargo test -p hsip-api auth::tests
```

### Integration Tests

```bash
# Start test environment
docker-compose -f docker-compose.dev.yml up -d

# Run full API tests
cargo test -p hsip-api --features integration-tests
```

### Manual Testing with curl

```bash
# 1. Login
TOKEN=$(curl -X POST http://localhost:8787/api/v2/auth/login \
  -H "Content-Type: application/json" \
  -d '{"username":"admin","password":"admin"}' \
  | jq -r '.access_token')

# 2. Use token to call protected endpoint
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8787/api/v2/messages/send \
  -d '{"recipient":"user@example.com","message_type":"alert","content":{}}'

# 3. Verify token info
curl -H "Authorization: Bearer $TOKEN" \
  http://localhost:8787/api/v2/auth/token/info
```

## Troubleshooting

### Common Errors

**Error:** `401 Unauthorized`
```
Cause: Invalid or expired token
Solution: Get new token via /auth/login
```

**Error:** `403 Forbidden`
```
Cause: Valid token but insufficient permissions
Solution: Request token with required roles/scopes
```

**Error:** `Invalid token signature`
```
Cause: JWT_SECRET mismatch between issuer and validator
Solution: Ensure same JWT_SECRET on all servers
```

**Error:** `Token has expired`
```
Cause: Token older than expires_in
Solution: Use /auth/token/refresh to get new token
```

### Debug Token Contents

```bash
# Install jwt-cli
cargo install jwt-cli

# Decode token (without verification)
jwt decode $TOKEN

# Verify and decode
jwt decode --secret "$JWT_SECRET" $TOKEN
```

### Logging

Enable debug logging:
```bash
export RUST_LOG=hsip_api::auth=debug

# View auth logs
docker logs -f hsip-api | grep auth
```

## Production Deployment

### 1. Generate Production Secrets

```bash
# Generate 256-bit secret
JWT_SECRET=$(openssl rand -base64 32)

# Store in environment or secrets manager
echo "JWT_SECRET=$JWT_SECRET" >> /etc/hsip/.env
```

### 2. Configure Environment

```bash
# Production settings
export JWT_SECRET="<generated-secret>"
export JWT_ISSUER="hsip-api-prod"
export JWT_AUDIENCE="hsip-clients-prod"
export DATABASE_URL="postgres://hsip:$DB_PASSWORD@postgres:5432/hsip_prod"
```

### 3. Enable HTTPS

```yaml
# docker-compose.yml
services:
  nginx:
    image: nginx:alpine
    ports:
      - "443:443"
    volumes:
      - ./nginx.conf:/etc/nginx/nginx.conf
      - /etc/letsencrypt:/etc/letsencrypt
```

### 4. Set Up User Accounts

```sql
-- Create admin user (in production database)
INSERT INTO users (username, password_hash, roles, created_at)
VALUES ('admin', '$argon2id$...', ARRAY['admin'], NOW());

-- Create API user
INSERT INTO users (username, password_hash, roles, created_at)
VALUES ('chase_api', '$argon2id$...', ARRAY['bank_admin'], NOW());
```

### 5. Monitor Authentication

```bash
# Failed login attempts
SELECT user_id, COUNT(*) as attempts, MAX(timestamp) as last_attempt
FROM audit_log
WHERE event_type = 'auth_failed'
AND timestamp > NOW() - INTERVAL '1 hour'
GROUP BY user_id
HAVING COUNT(*) > 5;

# Active API keys
SELECT metadata->>'key_id' as key_id,
       metadata->>'name' as name,
       timestamp
FROM audit_log
WHERE event_type = 'api_key_created'
ORDER BY timestamp DESC;
```

## Future Enhancements

- [ ] OAuth 2.0 / OpenID Connect support
- [ ] Multi-factor authentication (MFA)
- [ ] API key rotation
- [ ] IP allowlisting per API key
- [ ] Token blacklisting / revocation
- [ ] Passwordless authentication (WebAuthn)
- [ ] SSO integration (SAML, Azure AD)
- [ ] Fine-grained resource-level permissions

## References

- JWT Specification: RFC 7519
- OAuth 2.0: RFC 6749
- OWASP Authentication Cheat Sheet
- NIST Digital Identity Guidelines (SP 800-63B)
- jsonwebtoken crate: https://docs.rs/jsonwebtoken/
