//! Plan catalogue. Limits are enforced by the API; prices are display-only
//! placeholders the business should set before selling.

use serde::Serialize;

pub const UNLIMITED: i64 = i64::MAX;

#[derive(Debug, Clone, Serialize)]
pub struct Plan {
    pub id: &'static str,
    pub name: &'static str,
    pub proofs_per_month: i64,
    pub streams: i64,
    pub seats: i64,
    pub evidence_max_proofs: i64,
    /// Display only; `None` = contact sales.
    pub price_usd_month: Option<i64>,
    pub highlights: &'static [&'static str],
}

pub const PLANS: &[Plan] = &[
    Plan {
        id: "free",
        name: "Starter",
        proofs_per_month: 1_000,
        streams: 2,
        seats: 2,
        evidence_max_proofs: 500,
        price_usd_month: Some(0),
        highlights: &[
            "Signed, hash-chained proofs",
            "Public verification links",
            "Evidence packs up to 500 proofs",
        ],
    },
    Plan {
        id: "team",
        name: "Team",
        proofs_per_month: 100_000,
        streams: 10,
        seats: 10,
        evidence_max_proofs: 5_000,
        price_usd_month: Some(499),
        highlights: &[
            "Everything in Starter",
            "Batch anchoring + OpenTimestamps",
            "Evidence packs up to 5,000 proofs",
            "Role-based access",
        ],
    },
    Plan {
        id: "business",
        name: "Business",
        proofs_per_month: 2_000_000,
        streams: 50,
        seats: 50,
        evidence_max_proofs: 20_000,
        price_usd_month: Some(2_500),
        highlights: &[
            "Everything in Team",
            "Evidence packs up to 20,000 proofs",
            "Priority support",
        ],
    },
    Plan {
        id: "enterprise",
        name: "Enterprise",
        proofs_per_month: UNLIMITED,
        streams: UNLIMITED,
        seats: UNLIMITED,
        evidence_max_proofs: 100_000,
        price_usd_month: None,
        highlights: &[
            "Unlimited proofs, streams and seats",
            "Self-hosted or dedicated deployment",
            "Custom retention and SLAs",
        ],
    },
];

pub fn get(id: &str) -> &'static Plan {
    PLANS.iter().find(|p| p.id == id).unwrap_or(&PLANS[0])
}

pub fn exists(id: &str) -> bool {
    PLANS.iter().any(|p| p.id == id)
}

/// Enterprise services catalogue shown in the console and on the landing
/// page. `status` is deliberately honest: `available` exists today,
/// `on_request` is delivered by our team, `roadmap` is not built yet.
#[derive(Debug, Clone, Serialize)]
pub struct Service {
    pub id: &'static str,
    pub name: &'static str,
    pub summary: &'static str,
    pub status: &'static str,
}

pub const SERVICES: &[Service] = &[
    Service {
        id: "dedicated",
        name: "Dedicated or self-hosted deployment",
        summary: "Run the platform in your own cloud or datacenter, with your own database and master key. Nothing leaves your network.",
        status: "available",
    },
    Service {
        id: "evidence",
        name: "Audit and regulator evidence packs",
        summary: "Signed, independently verifiable exports of any period, with a printable report. Larger pack sizes on Enterprise.",
        status: "available",
    },
    Service {
        id: "bitcoin_anchor",
        name: "Bitcoin-backed timestamps",
        summary: "Commit batch roots to Bitcoin through OpenTimestamps so history cannot be rewritten, even by us.",
        status: "available",
    },
    Service {
        id: "integration",
        name: "Integration and onboarding support",
        summary: "We help decide what to record, wire up your first systems, and design what an auditor will ask for.",
        status: "on_request",
    },
    Service {
        id: "support_sla",
        name: "Priority support and SLA",
        summary: "Named contact, response-time commitments and incident escalation.",
        status: "on_request",
    },
    Service {
        id: "retention",
        name: "Custom retention and data residency",
        summary: "Keep records where and for as long as your policies require. Delivered through dedicated deployment.",
        status: "on_request",
    },
    Service {
        id: "security_review",
        name: "Security questionnaire support",
        summary: "We answer vendor-security questionnaires and walk your team through the design and its limits.",
        status: "on_request",
    },
    Service {
        id: "byok",
        name: "Customer-held signing keys",
        summary: "Your organization signs its own records so the platform never holds your key. Planned.",
        status: "roadmap",
    },
    Service {
        id: "sso",
        name: "SSO (SAML / OIDC) and SCIM",
        summary: "Sign in with your identity provider and provision people automatically. Planned.",
        status: "roadmap",
    },
];

pub fn service_exists(id: &str) -> bool {
    SERVICES.iter().any(|s| s.id == id)
}
