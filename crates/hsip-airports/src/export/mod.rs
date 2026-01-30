pub mod json;
pub mod markdown;

pub use json::JsonExporter;
pub use markdown::MarkdownExporter;

use crate::{Result, ComplianceGap};

/// Trait for exporting gap analysis reports
pub trait Exporter {
    /// Exports gaps to a string
    fn export(&self, gaps: &[ComplianceGap]) -> Result<String>;

    /// Exports to file
    fn export_to_file(&self, gaps: &[ComplianceGap], path: &std::path::Path) -> Result<()> {
        let content = self.export(gaps)?;
        std::fs::write(path, content)?;
        Ok(())
    }
}
