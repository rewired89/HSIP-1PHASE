//! Audit Log Export
//!
//! Export audit logs for compliance, regulators, and forensics

use crate::{AuditLog, Result};
use serde::Serialize;
use std::io::Write;

/// Export format
#[derive(Debug, Clone, Copy)]
pub enum ExportFormat {
    Json,
    JsonPretty,
    Csv,
}

/// Audit log exporter
pub struct AuditExporter;

impl AuditExporter {
    /// Export audit log to JSON
    pub fn export_json(log: &AuditLog, pretty: bool) -> Result<String> {
        let json = if pretty {
            serde_json::to_string_pretty(log)?
        } else {
            serde_json::to_string(log)?
        };

        Ok(json)
    }

    /// Export audit log to CSV
    pub fn export_csv(log: &AuditLog) -> Result<String> {
        let mut wtr = csv::Writer::from_writer(vec![]);

        // Write header
        wtr.write_record(&[
            "sequence_number",
            "timestamp",
            "event_type",
            "message_hash",
            "sender_identity",
            "recipient_identity",
            "verification_result",
            "entry_hash",
        ])?;

        // Write entries
        for signed_entry in &log.entries {
            let entry = &signed_entry.entry;

            wtr.write_record(&[
                entry.sequence_number.to_string(),
                entry.timestamp.to_rfc3339(),
                format!("{:?}", entry.event_type),
                hex::encode(entry.message_hash),
                entry.sender_identity.clone(),
                entry.recipient_identity.clone(),
                entry.verification_result.map_or("N/A".to_string(), |v| format!("{:?}", v)),
                hex::encode(entry.compute_hash()),
            ])?;
        }

        wtr.flush()?;

        let csv_bytes = wtr.into_inner().unwrap();
        Ok(String::from_utf8(csv_bytes).unwrap())
    }

    /// Export to specified format
    pub fn export(log: &AuditLog, format: ExportFormat) -> Result<String> {
        match format {
            ExportFormat::Json => Self::export_json(log, false),
            ExportFormat::JsonPretty => Self::export_json(log, true),
            ExportFormat::Csv => Self::export_csv(log),
        }
    }

    /// Export with integrity proof
    pub fn export_with_proof(log: &AuditLog, format: ExportFormat) -> Result<ExportPackage> {
        let data = Self::export(log, format)?;

        // Compute overall hash
        let data_hash = *blake3::hash(data.as_bytes()).as_bytes();

        Ok(ExportPackage {
            format,
            data,
            data_hash,
            entry_count: log.entry_count(),
            head_hash: log.head_hash(),
            signing_key_public: log.signing_key_public,
        })
    }
}

/// Export package with integrity proof
#[derive(Debug, Clone, Serialize)]
pub struct ExportPackage {
    #[serde(skip)]
    pub format: ExportFormat,
    pub data: String,
    pub data_hash: [u8; 32],
    pub entry_count: usize,
    pub head_hash: Option<[u8; 32]>,
    pub signing_key_public: [u8; 32],
}

impl ExportPackage {
    /// Verify integrity of exported data
    pub fn verify_integrity(&self) -> bool {
        let computed_hash = *blake3::hash(self.data.as_bytes()).as_bytes();
        computed_hash == self.data_hash
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{AuditEntry, AuditEventType};
    use ed25519_dalek::SigningKey;
    use rand::rngs::OsRng;

    #[test]
    fn test_json_export() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let mut log = AuditLog::new("test_log".to_string(), &signing_key);

        let entry = AuditEntry::new(
            1,
            [0u8; 32],
            AuditEventType::MessageSent,
            [1u8; 32],
            "sender".to_string(),
            "recipient".to_string(),
        );

        log.append(entry, &signing_key).unwrap();

        let json = AuditExporter::export_json(&log, false).unwrap();
        assert!(json.contains("test_log"));
    }

    #[test]
    fn test_csv_export() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let mut log = AuditLog::new("test_log".to_string(), &signing_key);

        let entry = AuditEntry::new(
            1,
            [0u8; 32],
            AuditEventType::MessageSent,
            [1u8; 32],
            "sender".to_string(),
            "recipient".to_string(),
        );

        log.append(entry, &signing_key).unwrap();

        let csv = AuditExporter::export_csv(&log).unwrap();
        assert!(csv.contains("sequence_number"));
        assert!(csv.contains("sender"));
    }

    #[test]
    fn test_export_with_proof() {
        let signing_key = SigningKey::generate(&mut OsRng);
        let mut log = AuditLog::new("test_log".to_string(), &signing_key);

        let entry = AuditEntry::new(
            1,
            [0u8; 32],
            AuditEventType::MessageSent,
            [1u8; 32],
            "sender".to_string(),
            "recipient".to_string(),
        );

        log.append(entry, &signing_key).unwrap();

        let package = AuditExporter::export_with_proof(&log, ExportFormat::Json).unwrap();
        assert!(package.verify_integrity());
    }
}
