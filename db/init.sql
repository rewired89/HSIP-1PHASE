-- HSIP Phase 2 Database Schema
-- PostgreSQL initialization script

-- Create extensions
CREATE EXTENSION IF NOT EXISTS "uuid-ossp";
CREATE EXTENSION IF NOT EXISTS "pgcrypto";

-- Audit log table
CREATE TABLE IF NOT EXISTS audit_log (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    event_type VARCHAR(50) NOT NULL,
    user_id VARCHAR(255),
    sender_domain VARCHAR(255),
    recipient VARCHAR(255),
    message_id VARCHAR(255),
    message_type VARCHAR(100),
    verification_status VARCHAR(50),
    signature_hex TEXT,
    public_key_hex TEXT,
    metadata JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_audit_timestamp ON audit_log(timestamp);
CREATE INDEX idx_audit_message_id ON audit_log(message_id);
CREATE INDEX idx_audit_user_id ON audit_log(user_id);
CREATE INDEX idx_audit_sender_domain ON audit_log(sender_domain);

-- Consent policies table
CREATE TABLE IF NOT EXISTS consent_policies (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    user_id VARCHAR(255) NOT NULL,
    sender_domain VARCHAR(255) NOT NULL,
    allowed_message_types TEXT[] NOT NULL,
    duration_days INTEGER,
    max_messages_per_day INTEGER,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ,
    revoked_at TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true,
    metadata JSONB
);

CREATE INDEX idx_consent_user ON consent_policies(user_id);
CREATE INDEX idx_consent_domain ON consent_policies(sender_domain);
CREATE INDEX idx_consent_active ON consent_policies(is_active) WHERE is_active = true;

-- Message verification cache
CREATE TABLE IF NOT EXISTS verification_cache (
    message_id VARCHAR(255) PRIMARY KEY,
    sender_domain VARCHAR(255) NOT NULL,
    verified BOOLEAN NOT NULL,
    verification_timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    verification_details JSONB
);

CREATE INDEX idx_verification_domain ON verification_cache(sender_domain);
CREATE INDEX idx_verification_expires ON verification_cache(expires_at);

-- Bank public keys registry
CREATE TABLE IF NOT EXISTS bank_public_keys (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    domain VARCHAR(255) UNIQUE NOT NULL,
    public_key_hex TEXT NOT NULL,
    algorithm VARCHAR(50) NOT NULL DEFAULT 'Ed25519',
    valid_from TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    valid_until TIMESTAMPTZ,
    is_active BOOLEAN DEFAULT true,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    updated_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metadata JSONB
);

CREATE INDEX idx_bank_keys_domain ON bank_public_keys(domain);
CREATE INDEX idx_bank_keys_active ON bank_public_keys(is_active) WHERE is_active = true;

-- Sessions table
CREATE TABLE IF NOT EXISTS sessions (
    id UUID PRIMARY KEY DEFAULT uuid_generate_v4(),
    session_id VARCHAR(255) UNIQUE NOT NULL,
    user_id VARCHAR(255),
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    last_activity TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    expires_at TIMESTAMPTZ NOT NULL,
    is_active BOOLEAN DEFAULT true,
    metadata JSONB
);

CREATE INDEX idx_sessions_user ON sessions(user_id);
CREATE INDEX idx_sessions_active ON sessions(is_active) WHERE is_active = true;

-- Metrics table (for business intelligence)
CREATE TABLE IF NOT EXISTS metrics (
    id BIGSERIAL PRIMARY KEY,
    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
    metric_name VARCHAR(100) NOT NULL,
    metric_value NUMERIC NOT NULL,
    tags JSONB,
    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
);

CREATE INDEX idx_metrics_timestamp ON metrics(timestamp);
CREATE INDEX idx_metrics_name ON metrics(metric_name);

-- Create a view for active consents
CREATE OR REPLACE VIEW active_consents AS
SELECT
    cp.*,
    CASE
        WHEN cp.expires_at IS NULL THEN true
        WHEN cp.expires_at > NOW() THEN true
        ELSE false
    END AS is_currently_valid
FROM consent_policies cp
WHERE cp.is_active = true
  AND cp.revoked_at IS NULL;

-- Create a function to clean up expired entries
CREATE OR REPLACE FUNCTION cleanup_expired_data()
RETURNS void AS $$
BEGIN
    -- Clean up expired verification cache
    DELETE FROM verification_cache WHERE expires_at < NOW();

    -- Deactivate expired consents
    UPDATE consent_policies
    SET is_active = false
    WHERE expires_at < NOW()
      AND is_active = true;

    -- Deactivate expired sessions
    UPDATE sessions
    SET is_active = false
    WHERE expires_at < NOW()
      AND is_active = true;
END;
$$ LANGUAGE plpgsql;

-- Grant permissions
GRANT ALL PRIVILEGES ON ALL TABLES IN SCHEMA public TO hsip;
GRANT ALL PRIVILEGES ON ALL SEQUENCES IN SCHEMA public TO hsip;
GRANT EXECUTE ON ALL FUNCTIONS IN SCHEMA public TO hsip;

-- Insert sample bank public key (for testing)
INSERT INTO bank_public_keys (domain, public_key_hex, algorithm, metadata)
VALUES (
    'testbank.com',
    'deadbeef0123456789abcdef0123456789abcdef0123456789abcdef01234567',
    'Ed25519',
    '{"environment": "test", "purpose": "demo"}'::jsonb
) ON CONFLICT (domain) DO NOTHING;
