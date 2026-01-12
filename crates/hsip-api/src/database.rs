//! Database layer for audit logging and consent management

use sqlx::{postgres::PgPoolOptions, PgPool, Row};
use chrono::{DateTime, Utc};
use serde_json::Value as JsonValue;
use uuid::Uuid;

/// Database connection pool manager
#[derive(Clone)]
pub struct Database {
    pool: PgPool,
}

impl Database {
    /// Create a new database connection from environment variable
    pub async fn new() -> Result<Self, sqlx::Error> {
        let database_url = std::env::var("DATABASE_URL")
            .unwrap_or_else(|_| "postgres://hsip:dev_password@localhost:5432/hsip_dev".to_string());

        Self::from_url(&database_url).await
    }

    /// Create a new database connection from URL
    pub async fn from_url(url: &str) -> Result<Self, sqlx::Error> {
        let pool = PgPoolOptions::new()
            .max_connections(20)
            .connect(url)
            .await?;

        Ok(Self { pool })
    }

    /// Write to audit log
    pub async fn audit_log(
        &self,
        event_type: &str,
        user_id: Option<&str>,
        sender_domain: Option<&str>,
        recipient: Option<&str>,
        message_id: Option<&str>,
        message_type: Option<&str>,
        verification_status: Option<&str>,
        signature_hex: Option<&str>,
        public_key_hex: Option<&str>,
        metadata: Option<JsonValue>,
    ) -> Result<i64, sqlx::Error> {
        let record = sqlx::query(
            r#"
            INSERT INTO audit_log (
                event_type, user_id, sender_domain, recipient,
                message_id, message_type, verification_status,
                signature_hex, public_key_hex, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9, $10)
            RETURNING id
            "#,
        )
        .bind(event_type)
        .bind(user_id)
        .bind(sender_domain)
        .bind(recipient)
        .bind(message_id)
        .bind(message_type)
        .bind(verification_status)
        .bind(signature_hex)
        .bind(public_key_hex)
        .bind(metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.get("id"))
    }

    /// Create consent policy
    pub async fn create_consent_policy(
        &self,
        user_id: &str,
        sender_domain: &str,
        allowed_message_types: &[String],
        duration_days: Option<i32>,
        max_messages_per_day: Option<i32>,
        metadata: Option<JsonValue>,
    ) -> Result<Uuid, sqlx::Error> {
        let expires_at = duration_days.map(|days| {
            Utc::now() + chrono::Duration::days(days as i64)
        });

        let record = sqlx::query(
            r#"
            INSERT INTO consent_policies (
                user_id, sender_domain, allowed_message_types,
                duration_days, max_messages_per_day, expires_at, metadata
            )
            VALUES ($1, $2, $3, $4, $5, $6, $7)
            RETURNING id
            "#,
        )
        .bind(user_id)
        .bind(sender_domain)
        .bind(allowed_message_types)
        .bind(duration_days)
        .bind(max_messages_per_day)
        .bind(expires_at)
        .bind(metadata)
        .fetch_one(&self.pool)
        .await?;

        Ok(record.get("id"))
    }

    /// Revoke consent policy
    pub async fn revoke_consent(
        &self,
        user_id: &str,
        sender_domain: &str,
    ) -> Result<u64, sqlx::Error> {
        let result = sqlx::query(
            r#"
            UPDATE consent_policies
            SET is_active = false, revoked_at = NOW()
            WHERE user_id = $1 AND sender_domain = $2 AND is_active = true
            "#,
        )
        .bind(user_id)
        .bind(sender_domain)
        .execute(&self.pool)
        .await?;

        Ok(result.rows_affected())
    }

    /// Check if consent is active
    pub async fn check_consent(
        &self,
        user_id: &str,
        sender_domain: &str,
        message_type: &str,
    ) -> Result<bool, sqlx::Error> {
        let record = sqlx::query(
            r#"
            SELECT COUNT(*) as count
            FROM consent_policies
            WHERE user_id = $1
                AND sender_domain = $2
                AND $3 = ANY(allowed_message_types)
                AND is_active = true
                AND (expires_at IS NULL OR expires_at > NOW())
            "#,
        )
        .bind(user_id)
        .bind(sender_domain)
        .bind(message_type)
        .fetch_one(&self.pool)
        .await?;

        let count: i64 = record.get("count");
        Ok(count > 0)
    }

    /// Get audit logs within date range
    pub async fn get_audit_logs(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        limit: i64,
    ) -> Result<Vec<AuditLogEntry>, sqlx::Error> {
        let records = sqlx::query_as::<_, AuditLogEntry>(
            r#"
            SELECT
                id, timestamp, event_type, user_id, sender_domain,
                recipient, message_id, message_type, verification_status,
                signature_hex, public_key_hex, metadata, created_at
            FROM audit_log
            WHERE timestamp >= $1 AND timestamp <= $2
            ORDER BY timestamp DESC
            LIMIT $3
            "#,
        )
        .bind(start_date)
        .bind(end_date)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;

        Ok(records)
    }

    /// Get audit statistics
    pub async fn get_audit_stats(
        &self,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
    ) -> Result<AuditStats, sqlx::Error> {
        let record = sqlx::query(
            r#"
            SELECT
                COUNT(*) as total_events,
                COUNT(DISTINCT message_id) as unique_messages,
                COUNT(CASE WHEN verification_status = 'verified' THEN 1 END) as verified_count,
                COUNT(CASE WHEN verification_status = 'failed' THEN 1 END) as failed_count,
                COUNT(CASE WHEN event_type = 'consent_granted' THEN 1 END) as consents_granted,
                COUNT(CASE WHEN event_type = 'consent_revoked' THEN 1 END) as consents_revoked
            FROM audit_log
            WHERE timestamp >= $1 AND timestamp <= $2
            "#,
        )
        .bind(start_date)
        .bind(end_date)
        .fetch_one(&self.pool)
        .await?;

        Ok(AuditStats {
            total_events: record.get("total_events"),
            unique_messages: record.get("unique_messages"),
            verified_count: record.get("verified_count"),
            failed_count: record.get("failed_count"),
            consents_granted: record.get("consents_granted"),
            consents_revoked: record.get("consents_revoked"),
        })
    }

    /// Health check
    pub async fn health_check(&self) -> Result<bool, sqlx::Error> {
        sqlx::query("SELECT 1")
            .fetch_one(&self.pool)
            .await?;
        Ok(true)
    }
}

/// Audit log entry from database
#[derive(Debug, Clone, sqlx::FromRow, serde::Serialize)]
pub struct AuditLogEntry {
    pub id: i64,
    pub timestamp: DateTime<Utc>,
    pub event_type: String,
    pub user_id: Option<String>,
    pub sender_domain: Option<String>,
    pub recipient: Option<String>,
    pub message_id: Option<String>,
    pub message_type: Option<String>,
    pub verification_status: Option<String>,
    pub signature_hex: Option<String>,
    pub public_key_hex: Option<String>,
    pub metadata: Option<JsonValue>,
    pub created_at: DateTime<Utc>,
}

/// Audit statistics
#[derive(Debug, Clone, serde::Serialize)]
pub struct AuditStats {
    pub total_events: i64,
    pub unique_messages: i64,
    pub verified_count: i64,
    pub failed_count: i64,
    pub consents_granted: i64,
    pub consents_revoked: i64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    #[ignore] // Requires running PostgreSQL
    async fn test_database_connection() {
        let db = Database::new().await;
        assert!(db.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_audit_log_write() {
        let db = Database::new().await.unwrap();

        let id = db.audit_log(
            "test_event",
            Some("test_user"),
            Some("chase.com"),
            Some("user@example.com"),
            Some("msg_test123"),
            Some("transaction_alert"),
            Some("verified"),
            None,
            None,
            Some(serde_json::json!({"test": true})),
        ).await;

        assert!(id.is_ok());
    }

    #[tokio::test]
    #[ignore]
    async fn test_consent_policy() {
        let db = Database::new().await.unwrap();

        let policy_id = db.create_consent_policy(
            "test_user",
            "chase.com",
            &["transaction_alert".to_string()],
            Some(30),
            Some(10),
            None,
        ).await;

        assert!(policy_id.is_ok());

        let has_consent = db.check_consent(
            "test_user",
            "chase.com",
            "transaction_alert",
        ).await;

        assert!(has_consent.unwrap());

        let revoked = db.revoke_consent("test_user", "chase.com").await;
        assert!(revoked.is_ok());
        assert_eq!(revoked.unwrap(), 1);
    }
}
