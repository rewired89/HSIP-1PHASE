use serde::{Deserialize, Serialize};

/// Recommended mitigation for a cybersecurity gap
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Mitigation {
    /// List of specific mitigation actions
    pub actions: Vec<String>,

    /// Implementation priority
    pub priority: ImplementationPriority,

    /// Estimated implementation complexity
    pub complexity: ImplementationComplexity,

    /// Estimated cost range (optional)
    pub estimated_cost: Option<String>,

    /// Estimated implementation timeframe
    pub estimated_timeframe: Option<String>,

    /// Required resources/expertise
    pub required_resources: Vec<String>,

    /// How HSIP can help address this gap (if applicable)
    pub hsip_solution: Option<String>,
}

/// Implementation priority
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Serialize, Deserialize)]
pub enum ImplementationPriority {
    /// Immediate action required (within 30 days)
    Immediate,

    /// Short-term (1-3 months)
    Short,

    /// Medium-term (3-6 months)
    Medium,

    /// Long-term (6+ months)
    Long,
}

/// Implementation complexity
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum ImplementationComplexity {
    /// Simple configuration changes, no development
    Low,

    /// Moderate effort, some integration work
    Medium,

    /// Significant effort, major architectural changes
    High,
}

impl Default for Mitigation {
    fn default() -> Self {
        Self {
            actions: Vec::new(),
            priority: ImplementationPriority::Medium,
            complexity: ImplementationComplexity::Medium,
            estimated_cost: None,
            estimated_timeframe: None,
            required_resources: Vec::new(),
            hsip_solution: None,
        }
    }
}

impl Mitigation {
    /// Creates a new mitigation
    pub fn new(priority: ImplementationPriority, complexity: ImplementationComplexity) -> Self {
        Self {
            actions: Vec::new(),
            priority,
            complexity,
            estimated_cost: None,
            estimated_timeframe: None,
            required_resources: Vec::new(),
            hsip_solution: None,
        }
    }

    /// Adds a mitigation action
    pub fn add_action(mut self, action: String) -> Self {
        self.actions.push(action);
        self
    }

    /// Sets estimated cost
    pub fn with_cost(mut self, cost: String) -> Self {
        self.estimated_cost = Some(cost);
        self
    }

    /// Sets estimated timeframe
    pub fn with_timeframe(mut self, timeframe: String) -> Self {
        self.estimated_timeframe = Some(timeframe);
        self
    }

    /// Adds required resource
    pub fn with_resource(mut self, resource: String) -> Self {
        self.required_resources.push(resource);
        self
    }

    /// Sets HSIP-specific solution
    pub fn with_hsip_solution(mut self, solution: String) -> Self {
        self.hsip_solution = Some(solution);
        self
    }
}

impl ImplementationPriority {
    /// Returns numeric score (higher = more urgent)
    pub fn score(&self) -> u8 {
        match self {
            Self::Immediate => 4,
            Self::Short => 3,
            Self::Medium => 2,
            Self::Long => 1,
        }
    }

    /// Returns days until implementation should be complete
    pub fn target_days(&self) -> u32 {
        match self {
            Self::Immediate => 30,
            Self::Short => 90,
            Self::Medium => 180,
            Self::Long => 365,
        }
    }
}

impl ImplementationComplexity {
    /// Returns estimated effort in person-days
    pub fn estimated_effort_days(&self) -> u32 {
        match self {
            Self::Low => 5,
            Self::Medium => 20,
            Self::High => 60,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mitigation_builder() {
        let mitigation = Mitigation::new(
            ImplementationPriority::Immediate,
            ImplementationComplexity::Low,
        )
        .add_action("Enable network segmentation".to_string())
        .add_action("Configure VLAN isolation".to_string())
        .with_cost("$10K-$25K".to_string())
        .with_timeframe("2-4 weeks".to_string())
        .with_resource("Network engineer".to_string())
        .with_hsip_solution("Use HSIP for consent-based vendor access control".to_string());

        assert_eq!(mitigation.actions.len(), 2);
        assert_eq!(mitigation.priority, ImplementationPriority::Immediate);
        assert!(mitigation.hsip_solution.is_some());
    }

    #[test]
    fn test_priority_ordering() {
        assert!(ImplementationPriority::Immediate > ImplementationPriority::Short);
        assert!(ImplementationPriority::Short > ImplementationPriority::Medium);
        assert!(ImplementationPriority::Medium > ImplementationPriority::Long);
    }

    #[test]
    fn test_target_days() {
        assert_eq!(ImplementationPriority::Immediate.target_days(), 30);
        assert_eq!(ImplementationPriority::Long.target_days(), 365);
    }
}
