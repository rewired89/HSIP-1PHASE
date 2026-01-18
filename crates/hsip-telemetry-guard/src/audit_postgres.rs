//! PostgreSQL-backed audit trail with write-once constraints
//!
//! Provides persistent, tamper-evident audit logs stored in PostgreSQL.
//! Write-once constraints prevent modification or deletion of audit entries.

#[cfg(feature = "postgres")]
use tokio_postgres::{Client, Config, NoTls};

use crate::{Decision, DecisionType, TelemetryIntent};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use tokio::sync::RwLock;

/// PostgreSQL audit entry (matches AuditEntry structure)
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct PgAuditEntry {
    pub entry_id: Vec<u8>,
    pub timestamp: DateTime<Utc>,
    pub decision: DecisionType,
    pub destination: String,
    pub intent: TelemetryIntent,
    pub reason: String,
    pub flow_id_prefix: String,
    pub prev_hash: Vec<u8>,
    pub entry_hash: Vec<u8>,
}

/// PostgreSQL audit log backend
#[cfg(feature = "postgres")]
pub struct PostgresAuditLog {
    client: Arc<RwLock<Option<Client>>>,
    connection_string: String,
}

#[cfg(feature = "postgres")]
impl PostgresAuditLog {
    /// Create a new PostgreSQL audit log
    pub fn new(connection_string: String) -> Self {
        Self {
            client: Arc::new(RwLock::new(None)),
            connection_string,
        }
    }

    /// Initialize database connection and schema
    pub async fn init(&self) -> Result<(), String> {
        let config = self.connection_string
            .parse::<Config>()
            .map_err(|e| format!("Invalid connection string: {}", e))?;

        let (client, connection) = config
            .connect(NoTls)
            .await
            .map_err(|e| format!("Failed to connect to PostgreSQL: {}", e))?;

        // Spawn connection handler
        tokio::spawn(async move {
            if let Err(e) = connection.await {
                eprintln!("PostgreSQL connection error: {}", e);
            }
        });

        // Create schema with write-once constraints
        self.create_schema(&client).await?;

        *self.client.write().await = Some(client);

        Ok(())
    }

    /// Create database schema with write-once constraints
    async fn create_schema(&self, client: &Client) -> Result<(), String> {
        // Create audit_log table with write-once constraints
        client
            .execute(
                "CREATE TABLE IF NOT EXISTS hsip_audit_log (
                    id BIGSERIAL PRIMARY KEY,
                    entry_id BYTEA NOT NULL UNIQUE,
                    timestamp TIMESTAMPTZ NOT NULL DEFAULT NOW(),
                    decision VARCHAR(50) NOT NULL,
                    destination TEXT NOT NULL,
                    intent VARCHAR(100) NOT NULL,
                    reason TEXT NOT NULL,
                    flow_id_prefix VARCHAR(100) NOT NULL,
                    prev_hash BYTEA NOT NULL,
                    entry_hash BYTEA NOT NULL,
                    created_at TIMESTAMPTZ NOT NULL DEFAULT NOW()
                )",
                &[],
            )
            .await
            .map_err(|e| format!("Failed to create audit_log table: {}", e))?;

        // Create write-once constraint: prevent UPDATE and DELETE
        // Use REVOKE to prevent modifications after insert
        client
            .execute(
                "DO $$
                BEGIN
                    IF NOT EXISTS (
                        SELECT 1 FROM pg_trigger
                        WHERE tgname = 'prevent_audit_modification'
                    ) THEN
                        CREATE OR REPLACE FUNCTION prevent_audit_modification()
                        RETURNS TRIGGER AS $trig$
                        BEGIN
                            IF TG_OP = 'UPDATE' OR TG_OP = 'DELETE' THEN
                                RAISE EXCEPTION 'Audit log entries are write-once and cannot be modified or deleted';
                            END IF;
                            RETURN NEW;
                        END;
                        $trig$ LANGUAGE plpgsql;

                        CREATE TRIGGER prevent_audit_modification
                        BEFORE UPDATE OR DELETE ON hsip_audit_log
                        FOR EACH ROW
                        EXECUTE FUNCTION prevent_audit_modification();
                    END IF;
                END $$;",
                &[],
            )
            .await
            .map_err(|e| format!("Failed to create write-once trigger: {}", e))?;

        // Create index for fast lookups
        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_audit_timestamp ON hsip_audit_log(timestamp DESC)",
                &[],
            )
            .await
            .map_err(|e| format!("Failed to create timestamp index: {}", e))?;

        client
            .execute(
                "CREATE INDEX IF NOT EXISTS idx_audit_destination ON hsip_audit_log(destination)",
                &[],
            )
            .await
            .map_err(|e| format!("Failed to create destination index: {}", e))?;

        Ok(())
    }

    /// Log a decision to PostgreSQL
    pub async fn log(&self, decision: &Decision) -> Result<Vec<u8>, String> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or_else(|| "PostgreSQL client not initialized".to_string())?;

        // Get previous hash from last entry
        let prev_hash = self.get_latest_hash(&client).await?;

        // Compute entry ID
        let mut id_hasher = blake3::Hasher::new();
        id_hasher.update(&decision.timestamp.timestamp_nanos_opt().unwrap_or(0).to_le_bytes());
        id_hasher.update(decision.flow_summary.flow_id_prefix.as_bytes());
        let entry_id = id_hasher.finalize().as_bytes().to_vec();

        // Compute entry hash (same logic as in-memory version)
        let mut hash_hasher = blake3::Hasher::new();
        hash_hasher.update(&entry_id);
        hash_hasher.update(&[decision.decision_type as u8]);
        hash_hasher.update(decision.flow_summary.destination.as_bytes());
        hash_hasher.update(&prev_hash);
        let entry_hash = hash_hasher.finalize().as_bytes().to_vec();

        // Insert into database
        client
            .execute(
                "INSERT INTO hsip_audit_log
                (entry_id, timestamp, decision, destination, intent, reason, flow_id_prefix, prev_hash, entry_hash)
                VALUES ($1, $2, $3, $4, $5, $6, $7, $8, $9)",
                &[
                    &entry_id,
                    &decision.timestamp,
                    &format!("{:?}", decision.decision_type),
                    &decision.flow_summary.destination,
                    &format!("{:?}", decision.flow_summary.intent),
                    &decision.primary_reason.description(),
                    &decision.flow_summary.flow_id_prefix,
                    &prev_hash,
                    &entry_hash,
                ],
            )
            .await
            .map_err(|e| format!("Failed to insert audit entry: {}", e))?;

        Ok(entry_id)
    }

    /// Get the latest entry hash for chain linking
    async fn get_latest_hash(&self, client: &Client) -> Result<Vec<u8>, String> {
        let row = client
            .query_opt(
                "SELECT entry_hash FROM hsip_audit_log ORDER BY id DESC LIMIT 1",
                &[],
            )
            .await
            .map_err(|e| format!("Failed to query latest hash: {}", e))?;

        Ok(row
            .map(|r| r.get::<_, Vec<u8>>(0))
            .unwrap_or_else(|| vec![0u8; 32]))
    }

    /// Verify chain integrity
    pub async fn verify_chain(&self) -> Result<bool, String> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or_else(|| "PostgreSQL client not initialized".to_string())?;

        let rows = client
            .query(
                "SELECT entry_id, decision, destination, prev_hash, entry_hash
                FROM hsip_audit_log ORDER BY id ASC",
                &[],
            )
            .await
            .map_err(|e| format!("Failed to query audit entries: {}", e))?;

        let mut expected_prev = vec![0u8; 32];

        for row in rows {
            let entry_id: Vec<u8> = row.get(0);
            let decision: String = row.get(1);
            let destination: String = row.get(2);
            let prev_hash: Vec<u8> = row.get(3);
            let entry_hash: Vec<u8> = row.get(4);

            // Verify chain link
            if prev_hash != expected_prev {
                return Ok(false);
            }

            // Verify entry integrity
            let mut hash_hasher = blake3::Hasher::new();
            hash_hasher.update(&entry_id);
            hash_hasher.update(decision.as_bytes());
            hash_hasher.update(destination.as_bytes());
            hash_hasher.update(&prev_hash);
            let computed_hash = hash_hasher.finalize().as_bytes().to_vec();

            if computed_hash != entry_hash {
                return Ok(false);
            }

            expected_prev = entry_hash;
        }

        Ok(true)
    }

    /// Get recent entries
    pub async fn recent(&self, limit: usize) -> Result<Vec<PgAuditEntry>, String> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or_else(|| "PostgreSQL client not initialized".to_string())?;

        let rows = client
            .query(
                "SELECT entry_id, timestamp, decision, destination, intent, reason,
                flow_id_prefix, prev_hash, entry_hash
                FROM hsip_audit_log ORDER BY timestamp DESC LIMIT $1",
                &[&(limit as i64)],
            )
            .await
            .map_err(|e| format!("Failed to query recent entries: {}", e))?;

        let mut entries = Vec::new();
        for row in rows {
            entries.push(PgAuditEntry {
                entry_id: row.get(0),
                timestamp: row.get(1),
                decision: self.parse_decision(&row.get::<_, String>(2)),
                destination: row.get(3),
                intent: self.parse_intent(&row.get::<_, String>(4)),
                reason: row.get(5),
                flow_id_prefix: row.get(6),
                prev_hash: row.get(7),
                entry_hash: row.get(8),
            });
        }

        Ok(entries)
    }

    /// Get entry count
    pub async fn len(&self) -> Result<usize, String> {
        let client_guard = self.client.read().await;
        let client = client_guard
            .as_ref()
            .ok_or_else(|| "PostgreSQL client not initialized".to_string())?;

        let row = client
            .query_one("SELECT COUNT(*) FROM hsip_audit_log", &[])
            .await
            .map_err(|e| format!("Failed to count entries: {}", e))?;

        Ok(row.get::<_, i64>(0) as usize)
    }

    /// Export as JSON
    pub async fn export_json(&self) -> Result<String, String> {
        let entries = self.recent(1000000).await?; // Export all entries
        serde_json::to_string_pretty(&entries)
            .map_err(|e| format!("Failed to serialize entries: {}", e))
    }

    fn parse_decision(&self, s: &str) -> DecisionType {
        match s {
            "Allow" => DecisionType::Allow,
            "Block" => DecisionType::Block,
            "Quarantine" => DecisionType::Quarantine,
            "AllowOnce" => DecisionType::AllowOnce,
            _ => DecisionType::Block,
        }
    }

    fn parse_intent(&self, s: &str) -> TelemetryIntent {
        match s {
            "CrashReport" => TelemetryIntent::CrashReport,
            "UsageAnalytics" => TelemetryIntent::UsageAnalytics,
            "Diagnostics" => TelemetryIntent::Diagnostics,
            "Advertising" => TelemetryIntent::Advertising,
            _ => TelemetryIntent::Unknown,
        }
    }
}

#[cfg(not(feature = "postgres"))]
pub struct PostgresAuditLog;

#[cfg(not(feature = "postgres"))]
impl PostgresAuditLog {
    pub fn new(_connection_string: String) -> Self {
        Self
    }

    pub async fn init(&self) -> Result<(), String> {
        Err("PostgreSQL support not enabled. Compile with --features postgres".to_string())
    }
}
