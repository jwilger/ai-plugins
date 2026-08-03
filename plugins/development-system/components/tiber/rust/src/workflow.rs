#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowDecision {
    pub frontier: u8,
    pub next: &'static str,
    pub evidence: &'static str,
    pub events: &'static str,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowProjection {
    frontier: u8,
}

impl WorkflowProjection {
    pub const fn initial() -> Self {
        Self { frontier: 0 }
    }

    pub fn decision(self) -> WorkflowDecision {
        match self.frontier {
            0 => WorkflowDecision {
                frontier: 0,
                next: "RecordDiscoveryEvidence",
                evidence: "artifact,observation,measurement",
                events: "DiscoveryEvidenceRecorded",
            },
            _ => unreachable!("a workflow projection only exposes known frontiers"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::WorkflowProjection;

    #[test]
    fn initial_projection_selects_only_discovery_evidence_recording() {
        assert_eq!(
            WorkflowProjection::initial().decision().next,
            "RecordDiscoveryEvidence"
        );
    }
}
