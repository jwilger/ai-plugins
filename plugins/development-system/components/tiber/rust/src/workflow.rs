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

impl Default for WorkflowProjection { fn default() -> Self { Self::initial() } }

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
            1 => WorkflowDecision { frontier: 1, next: "FormProductHypothesis", evidence: "derived", events: "ProductHypothesisFormed" },
            _ => unreachable!("a workflow projection only exposes known frontiers"),
        }
    }

    pub fn apply(mut self, event: &WorkflowEvent) -> Self {
        match event { WorkflowEvent::DiscoveryEvidenceRecordedV1 { .. } if self.frontier == 0 => self.frontier = 1, _ => {} }
        self
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
use eventcore::{CommandError, CommandLogic, CommandStreams, Event, NewEvents, StreamDeclarations, StreamId};
use serde::{Deserialize, Serialize};

pub const WORKFLOW_STREAM: &str = "workflow-local";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEvent {
    DiscoveryEvidenceRecordedV1 { workflow_id: StreamId, command: String, event: String, observation: String },
}

impl Event for WorkflowEvent {
    fn stream_id(&self) -> &StreamId { match self { Self::DiscoveryEvidenceRecordedV1 { workflow_id, .. } => workflow_id } }
    fn event_type_name() -> &'static str { "TiberWorkflowEvent" }
}

pub struct RecordDiscoveryEvidence { pub workflow_id: StreamId, pub observation: String }

impl CommandStreams for RecordDiscoveryEvidence {
    fn stream_declarations(&self) -> StreamDeclarations { StreamDeclarations::try_from_streams(vec![self.workflow_id.clone()]).expect("valid workflow stream") }
}

impl CommandLogic for RecordDiscoveryEvidence {
    type Event = WorkflowEvent;
    type State = WorkflowProjection;
    fn apply(&self, state: WorkflowProjection, event: &WorkflowEvent) -> WorkflowProjection { state.apply(event) }
    fn handle(&self, state: WorkflowProjection) -> Result<NewEvents<WorkflowEvent>, CommandError> {
        if state.frontier != 0 { return Err(CommandError::from("stale frontier")); }
        Ok(vec![WorkflowEvent::DiscoveryEvidenceRecordedV1 { workflow_id: self.workflow_id.clone(), command: "RecordDiscoveryEvidence".to_owned(), event: "DiscoveryEvidenceRecorded".to_owned(), observation: self.observation.clone() }].into())
    }
}
