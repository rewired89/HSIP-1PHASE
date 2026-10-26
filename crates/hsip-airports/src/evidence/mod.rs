pub mod source;
pub mod collector;
pub mod provenance;

pub use source::{EvidenceRef, SourceType, Accessibility, EvidenceGrade};
pub use collector::Collector;
pub use provenance::ProvenanceTracker;
