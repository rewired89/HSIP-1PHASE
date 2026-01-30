pub mod incident_mode;
pub mod audit_chain;

pub use incident_mode::IncidentModePolicy;
pub use audit_chain::CourtGradeAuditBundle;

// Additional feature modules to be implemented:
// - hardware_identity.rs (YubiKey/TPM support)
// - consent_templates.rs (role-based consent for airports)
// - soc_export.rs (CEF/LEEF SIEM adapters)
