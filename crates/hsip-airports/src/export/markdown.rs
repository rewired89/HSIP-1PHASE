use crate::{Result, ComplianceGap};
use super::Exporter;

/// Markdown exporter for gap analysis reports
pub struct MarkdownExporter;

impl MarkdownExporter {
    pub fn new() -> Self {
        Self
    }
}

impl Exporter for MarkdownExporter {
    fn export(&self, gaps: &[ComplianceGap]) -> Result<String> {
        let mut md = String::new();

        md.push_str("# Airport Cybersecurity Gap Analysis Report\n\n");
        md.push_str(&format!("**Total Gaps**: {}\n\n", gaps.len()));

        // Summary by severity
        let critical = gaps.iter().filter(|g| matches!(g.severity, crate::domain::Severity::Critical)).count();
        let high = gaps.iter().filter(|g| matches!(g.severity, crate::domain::Severity::High)).count();
        let medium = gaps.iter().filter(|g| matches!(g.severity, crate::domain::Severity::Medium)).count();
        let low = gaps.iter().filter(|g| matches!(g.severity, crate::domain::Severity::Low)).count();

        md.push_str("## Summary by Severity\n\n");
        md.push_str(&format!("- 🔴 **Critical**: {}\n", critical));
        md.push_str(&format!("- 🟠 **High**: {}\n", high));
        md.push_str(&format!("- 🟡 **Medium**: {}\n", medium));
        md.push_str(&format!("- 🟢 **Low**: {}\n\n", low));

        md.push_str("---\n\n");

        // Individual gap details
        for gap in gaps {
            let severity_icon = match gap.severity {
                crate::domain::Severity::Critical => "🔴",
                crate::domain::Severity::High => "🟠",
                crate::domain::Severity::Medium => "🟡",
                crate::domain::Severity::Low => "🟢",
            };

            md.push_str(&format!("## {} {} - {}\n\n", severity_icon, gap.gap_id, gap.title));

            md.push_str(&format!("**Severity**: {:?}  \n", gap.severity));
            md.push_str(&format!("**Category**: {:?}  \n", gap.category));
            md.push_str(&format!("**Confidence**: {}%  \n", gap.confidence));
            md.push_str(&format!("**Discovered**: {}  \n\n", gap.discovered_at.format("%Y-%m-%d %H:%M UTC")));

            md.push_str("### Description\n\n");
            md.push_str(&format!("{}\n\n", gap.description));

            // Affected systems
            if !gap.affected_systems.is_empty() {
                md.push_str("### Affected Systems\n\n");
                for system in &gap.affected_systems {
                    md.push_str(&format!("- {:?}\n", system));
                }
                md.push_str("\n");
            }

            // Coverage deficit
            md.push_str("### Coverage Deficit\n\n");
            md.push_str(&format!("{}\n\n", gap.coverage_deficit));

            // Framework coverage
            if !gap.current_coverage.is_empty() {
                md.push_str("### Regulatory Framework Coverage\n\n");
                for coverage in &gap.current_coverage {
                    md.push_str(&format!("- **{}**: {:?}", coverage.framework, coverage.coverage_level));
                    if let Some(controls) = &coverage.controls {
                        md.push_str(&format!(" ({})", controls.join(", ")));
                    }
                    md.push_str("\n");
                }
                md.push_str("\n");
            }

            // Evidence
            if !gap.evidence.is_empty() {
                md.push_str("### Evidence Sources\n\n");
                for (idx, evidence) in gap.evidence.iter().enumerate() {
                    md.push_str(&format!("{}. **{}** (Grade {:?})\n", idx + 1, evidence.title, evidence.evidence_grade));
                    md.push_str(&format!("   - Type: {:?}\n", evidence.source_type));
                    md.push_str(&format!("   - Excerpt: \"{}\"\n", evidence.excerpt));
                    if let Some(url) = &evidence.url {
                        md.push_str(&format!("   - URL: {}\n", url));
                    }
                    if let Some(section) = &evidence.section {
                        md.push_str(&format!("   - Reference: {}\n", section));
                    }
                }
                md.push_str("\n");
            }

            // Threat vectors
            if !gap.threat_vectors.is_empty() {
                md.push_str("### Threat Vectors\n\n");
                for threat in &gap.threat_vectors {
                    md.push_str(&format!("- **{}**: {}\n", threat.name, threat.description));
                }
                md.push_str("\n");
            }

            // Mitigation
            md.push_str("### Recommended Mitigation\n\n");
            md.push_str(&format!("**Priority**: {:?}  \n", gap.recommended_mitigation.priority));
            md.push_str(&format!("**Complexity**: {:?}  \n\n", gap.recommended_mitigation.complexity));

            if !gap.recommended_mitigation.actions.is_empty() {
                md.push_str("**Actions**:\n\n");
                for action in &gap.recommended_mitigation.actions {
                    md.push_str(&format!("- {}\n", action));
                }
                md.push_str("\n");
            }

            if let Some(hsip_solution) = &gap.recommended_mitigation.hsip_solution {
                md.push_str(&format!("**HSIP Solution**: {}\n\n", hsip_solution));
            }

            if let Some(timeframe) = &gap.recommended_mitigation.estimated_timeframe {
                md.push_str(&format!("**Estimated Timeframe**: {}\n\n", timeframe));
            }

            md.push_str("---\n\n");
        }

        md.push_str(&format!("\n*Report generated: {}*\n", chrono::Utc::now().format("%Y-%m-%d %H:%M:%S UTC")));

        Ok(md)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{GapCategory, Severity};
    use crate::AirportSystem;

    #[test]
    fn test_markdown_export() {
        let gap = ComplianceGap::new(
            "Test Gap".to_string(),
            "Test Description".to_string(),
            GapCategory::Technical,
            vec![AirportSystem::SCADA],
        );

        let exporter = MarkdownExporter::new();
        let markdown = exporter.export(&[gap]).unwrap();

        assert!(markdown.contains("# Airport Cybersecurity Gap Analysis Report"));
        assert!(markdown.contains("Test Gap"));
        assert!(markdown.contains("SCADA"));
        assert!(markdown.contains("Total Gaps"));
    }

    #[test]
    fn test_severity_summary() {
        let gaps = vec![
            ComplianceGap::new(
                "Critical Gap".to_string(),
                "Desc".to_string(),
                GapCategory::Technical,
                vec![AirportSystem::SCADA],
            ).with_severity(Severity::Critical),
            ComplianceGap::new(
                "High Gap".to_string(),
                "Desc".to_string(),
                GapCategory::Technical,
                vec![AirportSystem::BHS],
            ).with_severity(Severity::High),
        ];

        let exporter = MarkdownExporter::new();
        let markdown = exporter.export(&gaps).unwrap();

        assert!(markdown.contains("**Critical**: 1"));
        assert!(markdown.contains("**High**: 1"));
    }
}
