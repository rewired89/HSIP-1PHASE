use serde::{Deserialize, Serialize};
use chrono::{DateTime, Utc};

/// Evidence reference with full provenance tracking
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct EvidenceRef {
    /// Unique source identifier
    pub source_id: String,

    /// Document/source title
    pub title: String,

    /// Specific section or page reference
    pub section: Option<String>,

    /// Direct quote or excerpt (max 250 chars)
    pub excerpt: String,

    /// URL (if available)
    pub url: Option<String>,

    /// SHA-256 hash of source document (for integrity)
    pub file_hash: Option<String>,

    /// When this evidence was retrieved
    pub retrieved_at: DateTime<Utc>,

    /// Type of source
    pub source_type: SourceType,

    /// Accessibility level
    pub accessibility: Accessibility,

    /// Evidence quality grade
    pub evidence_grade: EvidenceGrade,
}

/// Type of evidence source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum SourceType {
    /// Government agency (NIST, CISA, TSA, FAA)
    Government,

    /// Standards body (ISO, IEC, ISA, ICAO)
    Standards,

    /// Peer-reviewed research (IEEE, ACM journals)
    PeerReviewed,

    /// Industry report (ACRP, white papers)
    IndustryReport,

    /// Security advisory (CISA KEV, vendor advisories)
    Advisory,

    /// News/media coverage
    Media,

    /// Internal analysis
    Internal,
}

/// Accessibility level of source
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum Accessibility {
    /// Publicly available, no restrictions
    Public,

    /// Requires authentication or purchase
    Restricted,

    /// Classified or confidential (cannot access)
    Classified,
}

/// Evidence quality grade
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum EvidenceGrade {
    /// Government/standards body primary source
    A,

    /// Peer-reviewed research
    B,

    /// Industry reports, advisories
    C,

    /// Secondary commentary, news coverage
    D,
}

impl EvidenceGrade {
    /// Returns numeric score for calculations
    pub fn score(&self) -> u8 {
        match self {
            Self::A => 4,
            Self::B => 3,
            Self::C => 2,
            Self::D => 1,
        }
    }

    /// Returns confidence multiplier (0.0 - 1.0)
    pub fn confidence_multiplier(&self) -> f32 {
        match self {
            Self::A => 1.0,
            Self::B => 0.85,
            Self::C => 0.65,
            Self::D => 0.40,
        }
    }
}

impl EvidenceRef {
    /// Creates a new evidence reference
    pub fn new(
        source_id: String,
        title: String,
        excerpt: String,
        source_type: SourceType,
    ) -> Self {
        let evidence_grade = match source_type {
            SourceType::Government | SourceType::Standards => EvidenceGrade::A,
            SourceType::PeerReviewed => EvidenceGrade::B,
            SourceType::IndustryReport | SourceType::Advisory => EvidenceGrade::C,
            SourceType::Media | SourceType::Internal => EvidenceGrade::D,
        };

        Self {
            source_id,
            title,
            section: None,
            excerpt,
            url: None,
            file_hash: None,
            retrieved_at: Utc::now(),
            source_type,
            accessibility: Accessibility::Public,
            evidence_grade,
        }
    }

    /// Sets section/page reference
    pub fn with_section(mut self, section: String) -> Self {
        self.section = Some(section);
        self
    }

    /// Sets URL
    pub fn with_url(mut self, url: String) -> Self {
        self.url = Some(url);
        self
    }

    /// Sets file hash for integrity verification
    pub fn with_file_hash(mut self, hash: String) -> Self {
        self.file_hash = Some(hash);
        self
    }

    /// Sets accessibility level
    pub fn with_accessibility(mut self, accessibility: Accessibility) -> Self {
        self.accessibility = accessibility;
        self
    }

    /// Returns true if evidence is publicly verifiable
    pub fn is_publicly_verifiable(&self) -> bool {
        self.accessibility == Accessibility::Public && self.url.is_some()
    }

    /// Returns true if evidence is high quality (A or B grade)
    pub fn is_high_quality(&self) -> bool {
        matches!(self.evidence_grade, EvidenceGrade::A | EvidenceGrade::B)
    }

    /// Returns citation string for reports
    pub fn citation(&self) -> String {
        let mut citation = format!("{}", self.title);

        if let Some(section) = &self.section {
            citation.push_str(&format!(", {}", section));
        }

        if let Some(url) = &self.url {
            citation.push_str(&format!(" ({})", url));
        }

        citation.push_str(&format!(", retrieved {}", self.retrieved_at.format("%Y-%m-%d")));

        citation
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_evidence_grade_auto_assignment() {
        let gov_ref = EvidenceRef::new(
            "NIST-CSF-2.0".to_string(),
            "NIST Cybersecurity Framework 2.0".to_string(),
            "Test excerpt".to_string(),
            SourceType::Government,
        );

        assert_eq!(gov_ref.evidence_grade, EvidenceGrade::A);
    }

    #[test]
    fn test_evidence_builder() {
        let evidence = EvidenceRef::new(
            "TSA-SD-001".to_string(),
            "TSA Security Directive 1580/82-2022-01".to_string(),
            "Airports must implement cybersecurity measures...".to_string(),
            SourceType::Government,
        )
        .with_section("Section 3.2.1".to_string())
        .with_url("https://www.tsa.gov/...".to_string())
        .with_file_hash("9f4a2c8d...".to_string());

        assert!(evidence.is_publicly_verifiable());
        assert!(evidence.is_high_quality());
    }

    #[test]
    fn test_citation_formatting() {
        let evidence = EvidenceRef::new(
            "TEST-001".to_string(),
            "Test Document".to_string(),
            "Excerpt".to_string(),
            SourceType::Government,
        )
        .with_section("Page 42".to_string())
        .with_url("https://example.com/doc.pdf".to_string());

        let citation = evidence.citation();
        assert!(citation.contains("Test Document"));
        assert!(citation.contains("Page 42"));
        assert!(citation.contains("https://example.com/doc.pdf"));
    }

    #[test]
    fn test_grade_confidence_multiplier() {
        assert_eq!(EvidenceGrade::A.confidence_multiplier(), 1.0);
        assert!(EvidenceGrade::D.confidence_multiplier() < 0.5);
    }
}
