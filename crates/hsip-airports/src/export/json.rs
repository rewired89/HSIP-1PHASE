use crate::{Result, ComplianceGap};
use super::Exporter;

/// JSON exporter for gap analysis reports
pub struct JsonExporter;

impl JsonExporter {
    pub fn new() -> Self {
        Self
    }
}

impl Exporter for JsonExporter {
    fn export(&self, gaps: &[ComplianceGap]) -> Result<String> {
        Ok(serde_json::to_string_pretty(gaps)?)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{GapCategory, Severity};
    use crate::AirportSystem;

    #[test]
    fn test_json_export() {
        let gap = ComplianceGap::new(
            "Test Gap".to_string(),
            "Description".to_string(),
            GapCategory::Technical,
            vec![AirportSystem::SCADA],
        );

        let exporter = JsonExporter::new();
        let json = exporter.export(&[gap]).unwrap();

        assert!(json.contains("Test Gap"));
        assert!(json.contains("SCADA"));
    }
}
