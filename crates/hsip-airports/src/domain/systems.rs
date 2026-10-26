use serde::{Deserialize, Serialize};

/// Airport subsystems covered in gap analysis
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum AirportSystem {
    /// Airport Operational Database - flight schedules, gate assignments, resource allocation
    AODB,

    /// Baggage Handling System - OT/SCADA-controlled conveyor belts and sorting
    BHS,

    /// Flight Information Display System - departure/arrival boards
    FIDS,

    /// Access Control System - badge readers, biometrics, mantraps, door locks
    ACS,

    /// SCADA/ICS - runway lighting, HVAC, fuel systems, jetbridge controls
    SCADA,

    /// ACARS/ARINC/SITA airline data networks
    AirlineDataNetworks,

    /// Departure Control Systems / Passenger Service Systems
    DCS_PSS,

    /// Public-facing WiFi and passenger networks
    PublicWiFi,

    /// Airport Collaborative Decision Making data feeds
    CDM,

    /// FAA System Wide Information Management integrations
    FAA_SWIM,

    /// CCTV and surveillance networks
    Surveillance,

    /// Emergency communication systems (PA, CIDS, fire alarms)
    EmergencyComms,

    /// Network infrastructure (routers, switches, VPN, segmentation)
    Network,

    /// Vendor/integrator remote access points
    VendorAccess,

    /// NOC/SOC monitoring systems
    Monitoring,
}

impl AirportSystem {
    /// Returns true if this is an Operational Technology (OT) system
    pub fn is_ot(&self) -> bool {
        matches!(
            self,
            Self::BHS | Self::SCADA | Self::EmergencyComms
        )
    }

    /// Returns true if this system is safety-critical
    pub fn is_safety_critical(&self) -> bool {
        matches!(
            self,
            Self::SCADA | Self::ACS | Self::EmergencyComms | Self::FAA_SWIM
        )
    }

    /// Returns true if this system has direct operational impact
    pub fn is_operational_critical(&self) -> bool {
        matches!(
            self,
            Self::AODB | Self::BHS | Self::FIDS | Self::DCS_PSS | Self::CDM
        )
    }

    /// Returns a human-readable description
    pub fn description(&self) -> &'static str {
        match self {
            Self::AODB => "Airport Operational Database - central hub for flight operations",
            Self::BHS => "Baggage Handling System - automated luggage processing",
            Self::FIDS => "Flight Information Display System - passenger information boards",
            Self::ACS => "Access Control System - physical security and access management",
            Self::SCADA => "SCADA/ICS - industrial control systems for airport infrastructure",
            Self::AirlineDataNetworks => "ACARS/ARINC/SITA - airline communication networks",
            Self::DCS_PSS => "Departure Control / Passenger Service Systems",
            Self::PublicWiFi => "Public WiFi - passenger-facing internet access",
            Self::CDM => "Collaborative Decision Making - operational coordination platform",
            Self::FAA_SWIM => "FAA SWIM - aviation data exchange with federal systems",
            Self::Surveillance => "CCTV and surveillance systems",
            Self::EmergencyComms => "Emergency communications - PA, alerts, fire systems",
            Self::Network => "Core network infrastructure",
            Self::VendorAccess => "Third-party vendor remote access points",
            Self::Monitoring => "NOC/SOC security monitoring systems",
        }
    }

    /// Returns typical attack surface for this system
    pub fn attack_surface(&self) -> &'static [&'static str] {
        match self {
            Self::AODB => &["SQL injection", "API abuse", "unauthorized data modification"],
            Self::BHS => &["PLC exploitation", "SCADA protocol attacks", "physical safety bypass"],
            Self::FIDS => &["content injection", "display hijacking", "DoS"],
            Self::ACS => &["credential cloning", "tailgating attacks", "access bypass"],
            Self::SCADA => &["Modbus/DNP3 attacks", "ladder logic tampering", "safety system bypass"],
            Self::AirlineDataNetworks => &["ACARS message spoofing", "network pivoting"],
            Self::DCS_PSS => &["reservation system manipulation", "PNR data theft"],
            Self::PublicWiFi => &["man-in-the-middle", "lateral movement to internal networks"],
            Self::CDM => &["false operational data injection", "decision manipulation"],
            Self::FAA_SWIM => &["flight data tampering", "unauthorized access to FAA systems"],
            Self::Surveillance => &["camera feed manipulation", "video deletion/tampering"],
            Self::EmergencyComms => &["false alarm injection", "emergency system disruption"],
            Self::Network => &["network segmentation bypass", "VLAN hopping"],
            Self::VendorAccess => &["credential theft", "privileged access abuse"],
            Self::Monitoring => &["blind spot creation", "alert suppression"],
        }
    }
}

impl std::fmt::Display for AirportSystem {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{:?}", self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_classification() {
        assert!(AirportSystem::SCADA.is_ot());
        assert!(AirportSystem::SCADA.is_safety_critical());
        assert!(!AirportSystem::PublicWiFi.is_ot());
        assert!(AirportSystem::AODB.is_operational_critical());
    }

    #[test]
    fn test_attack_surface() {
        let surfaces = AirportSystem::BHS.attack_surface();
        assert!(surfaces.contains(&"PLC exploitation"));
    }
}
