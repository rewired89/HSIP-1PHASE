use crate::{Result, AirportError};
use crate::evidence::{EvidenceRef, SourceType, Accessibility};
use sha2::{Sha256, Digest};
use std::path::PathBuf;

/// Document collector for cybersecurity sources
pub struct Collector {
    /// Local storage path for downloaded documents
    pub storage_path: PathBuf,

    /// HTTP client for fetching documents
    client: reqwest::Client,
}

impl Collector {
    /// Creates a new collector
    pub fn new(storage_path: PathBuf) -> Result<Self> {
        std::fs::create_dir_all(&storage_path)?;

        let client = reqwest::Client::builder()
            .user_agent("HSIP-Airports/0.1.0")
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .map_err(|e| AirportError::EvidenceError(e.to_string()))?;

        Ok(Self {
            storage_path,
            client,
        })
    }

    /// Fetches NIST Cybersecurity Framework 2.0
    pub async fn fetch_nist_csf(&self) -> Result<EvidenceRef> {
        let url = "https://nvlpubs.nist.gov/nistpubs/CSWP/NIST.CSWP.29.pdf";
        let source_id = "NIST-CSF-2.0";

        let evidence = self.fetch_document(
            source_id,
            "NIST Cybersecurity Framework 2.0",
            url,
            SourceType::Government,
        ).await?;

        Ok(evidence)
    }

    /// Fetches NIST SP 800-53 Rev 5
    pub async fn fetch_nist_800_53(&self) -> Result<EvidenceRef> {
        let url = "https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-53r5.pdf";
        let source_id = "NIST-SP-800-53-REV5";

        self.fetch_document(
            source_id,
            "NIST SP 800-53 Rev 5: Security and Privacy Controls",
            url,
            SourceType::Government,
        ).await
    }

    /// Fetches NIST SP 800-82
    pub async fn fetch_nist_800_82(&self) -> Result<EvidenceRef> {
        let url = "https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-82r3.pdf";
        let source_id = "NIST-SP-800-82-REV3";

        self.fetch_document(
            source_id,
            "NIST SP 800-82 Rev 3: Guide to ICS Security",
            url,
            SourceType::Government,
        ).await
    }

    /// Generic document fetcher with provenance tracking
    async fn fetch_document(
        &self,
        source_id: &str,
        title: &str,
        url: &str,
        source_type: SourceType,
    ) -> Result<EvidenceRef> {
        // Check if already downloaded
        let filename = format!("{}.pdf", source_id);
        let filepath = self.storage_path.join(&filename);

        let file_hash = if filepath.exists() {
            // Already have it, compute hash
            self.compute_file_hash(&filepath)?
        } else {
            // Download it
            tracing::info!("Fetching {} from {}", title, url);

            let response = self.client.get(url)
                .send()
                .await
                .map_err(|e| AirportError::EvidenceError(format!("Failed to fetch {}: {}", url, e)))?;

            if !response.status().is_success() {
                return Err(AirportError::EvidenceError(
                    format!("HTTP {} when fetching {}", response.status(), url)
                ));
            }

            let bytes = response.bytes().await
                .map_err(|e| AirportError::EvidenceError(format!("Failed to read response: {}", e)))?;

            // Save to disk
            std::fs::write(&filepath, &bytes)?;

            // Compute hash
            let hash = self.hash_bytes(&bytes);
            tracing::info!("Downloaded {} ({} bytes, hash: {})", title, bytes.len(), &hash[..16]);
            hash
        };

        Ok(EvidenceRef::new(
            source_id.to_string(),
            title.to_string(),
            format!("Downloaded from {}", url), // Excerpt placeholder
            source_type,
        )
        .with_url(url.to_string())
        .with_file_hash(file_hash))
    }

    /// Computes SHA-256 hash of a file
    fn compute_file_hash(&self, path: &PathBuf) -> Result<String> {
        let bytes = std::fs::read(path)?;
        Ok(self.hash_bytes(&bytes))
    }

    /// Computes SHA-256 hash of bytes
    fn hash_bytes(&self, bytes: &[u8]) -> String {
        let mut hasher = Sha256::new();
        hasher.update(bytes);
        hex::encode(hasher.finalize())
    }

    /// Lists all known public cybersecurity sources (metadata only)
    pub fn list_known_sources(&self) -> Vec<KnownSource> {
        vec![
            KnownSource {
                id: "NIST-CSF-2.0".to_string(),
                title: "NIST Cybersecurity Framework 2.0".to_string(),
                url: "https://nvlpubs.nist.gov/nistpubs/CSWP/NIST.CSWP.29.pdf".to_string(),
                source_type: SourceType::Government,
                accessibility: Accessibility::Public,
            },
            KnownSource {
                id: "NIST-SP-800-53-REV5".to_string(),
                title: "NIST SP 800-53 Rev 5".to_string(),
                url: "https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-53r5.pdf".to_string(),
                source_type: SourceType::Government,
                accessibility: Accessibility::Public,
            },
            KnownSource {
                id: "NIST-SP-800-82-REV3".to_string(),
                title: "NIST SP 800-82 Rev 3: ICS Security".to_string(),
                url: "https://nvlpubs.nist.gov/nistpubs/SpecialPublications/NIST.SP.800-82r3.pdf".to_string(),
                source_type: SourceType::Government,
                accessibility: Accessibility::Public,
            },
            KnownSource {
                id: "TSA-SD-1580-82-2022-01".to_string(),
                title: "TSA Security Directive 1580/82-2022-01".to_string(),
                url: "https://www.tsa.gov/".to_string(), // Placeholder - actual directives may be restricted
                source_type: SourceType::Government,
                accessibility: Accessibility::Restricted, // Note: May require FOIA or specific access
            },
            KnownSource {
                id: "CISA-KEV".to_string(),
                title: "CISA Known Exploited Vulnerabilities Catalog".to_string(),
                url: "https://www.cisa.gov/known-exploited-vulnerabilities-catalog".to_string(),
                source_type: SourceType::Advisory,
                accessibility: Accessibility::Public,
            },
        ]
    }
}

/// Known source metadata
#[derive(Debug, Clone)]
pub struct KnownSource {
    pub id: String,
    pub title: String,
    pub url: String,
    pub source_type: SourceType,
    pub accessibility: Accessibility,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    #[test]
    fn test_collector_creation() {
        let dir = tempdir().unwrap();
        let collector = Collector::new(dir.path().to_path_buf()).unwrap();
        assert!(dir.path().exists());
    }

    #[test]
    fn test_hash_computation() {
        let dir = tempdir().unwrap();
        let collector = Collector::new(dir.path().to_path_buf()).unwrap();

        let test_data = b"test data";
        let hash = collector.hash_bytes(test_data);

        // SHA-256 of "test data"
        assert_eq!(hash, "916f0027a575074ce72a331777c3478d6513f786a591bd892da1a577bf2335f9");
    }

    #[test]
    fn test_known_sources_list() {
        let dir = tempdir().unwrap();
        let collector = Collector::new(dir.path().to_path_buf()).unwrap();
        let sources = collector.list_known_sources();

        assert!(!sources.is_empty());
        assert!(sources.iter().any(|s| s.id == "NIST-CSF-2.0"));
    }
}
