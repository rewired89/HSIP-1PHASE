use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};
use std::collections::HashMap;
use crate::evidence::EvidenceRef;
use crate::Result;

/// Tracks provenance of all evidence used in gap analysis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProvenanceTracker {
    /// Map of source_id -> full evidence reference
    pub sources: HashMap<String, EvidenceRef>,

    /// Track when provenance data was last updated
    pub last_updated: DateTime<Utc>,
}

impl ProvenanceTracker {
    /// Creates a new provenance tracker
    pub fn new() -> Self {
        Self {
            sources: HashMap::new(),
            last_updated: Utc::now(),
        }
    }

    /// Adds an evidence source
    pub fn add_source(&mut self, evidence: EvidenceRef) {
        self.sources.insert(evidence.source_id.clone(), evidence);
        self.last_updated = Utc::now();
    }

    /// Gets an evidence source by ID
    pub fn get_source(&self, source_id: &str) -> Option<&EvidenceRef> {
        self.sources.get(source_id)
    }

    /// Returns all sources
    pub fn all_sources(&self) -> Vec<&EvidenceRef> {
        self.sources.values().collect()
    }

    /// Returns count of high-quality sources (Grade A or B)
    pub fn high_quality_count(&self) -> usize {
        self.sources.values()
            .filter(|e| e.is_high_quality())
            .count()
    }

    /// Returns count of publicly verifiable sources
    pub fn publicly_verifiable_count(&self) -> usize {
        self.sources.values()
            .filter(|e| e.is_publicly_verifiable())
            .count()
    }

    /// Exports provenance data as JSON
    pub fn export_json(&self) -> Result<String> {
        Ok(serde_json::to_string_pretty(self)?)
    }

    /// Loads provenance data from JSON
    pub fn from_json(json: &str) -> Result<Self> {
        Ok(serde_json::from_str(json)?)
    }
}

impl Default for ProvenanceTracker {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::evidence::SourceType;

    #[test]
    fn test_provenance_tracking() {
        let mut tracker = ProvenanceTracker::new();

        let evidence1 = EvidenceRef::new(
            "NIST-CSF".to_string(),
            "NIST CSF".to_string(),
            "Excerpt".to_string(),
            SourceType::Government,
        );

        let evidence2 = EvidenceRef::new(
            "TEST-002".to_string(),
            "Industry Report".to_string(),
            "Excerpt".to_string(),
            SourceType::IndustryReport,
        );

        tracker.add_source(evidence1);
        tracker.add_source(evidence2);

        assert_eq!(tracker.sources.len(), 2);
        assert!(tracker.get_source("NIST-CSF").is_some());
    }

    #[test]
    fn test_quality_metrics() {
        let mut tracker = ProvenanceTracker::new();

        // High quality
        tracker.add_source(EvidenceRef::new(
            "GOV-001".to_string(),
            "Gov Source".to_string(),
            "Test".to_string(),
            SourceType::Government,
        ));

        // Low quality
        tracker.add_source(EvidenceRef::new(
            "MEDIA-001".to_string(),
            "Media Source".to_string(),
            "Test".to_string(),
            SourceType::Media,
        ));

        assert_eq!(tracker.high_quality_count(), 1);
    }

    #[test]
    fn test_json_serialization() {
        let mut tracker = ProvenanceTracker::new();
        tracker.add_source(EvidenceRef::new(
            "TEST".to_string(),
            "Test".to_string(),
            "Test".to_string(),
            SourceType::Government,
        ));

        let json = tracker.export_json().unwrap();
        let loaded = ProvenanceTracker::from_json(&json).unwrap();

        assert_eq!(loaded.sources.len(), 1);
    }
}
