use std::fmt;

const RECORD_DISCOVERY_EVIDENCE: &str = "RecordDiscoveryEvidence";
const FORM_PRODUCT_HYPOTHESIS: &str = "FormProductHypothesis";
const DISCOVERY_EVIDENCE_RECORDED: &str = "DiscoveryEvidenceRecorded";
const PRODUCT_HYPOTHESIS_FORMED: &str = "ProductHypothesisFormed";

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct WorkflowReplayError(&'static str);

impl fmt::Display for WorkflowReplayError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.0)
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowCommandName(String);

impl WorkflowCommandName {
    fn parse(value: &str) -> Result<Self, WorkflowReplayError> {
        match value {
            RECORD_DISCOVERY_EVIDENCE | FORM_PRODUCT_HYPOTHESIS => Ok(Self(value.to_owned())),
            _ => Err(WorkflowReplayError("event_unknown")),
        }
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct WorkflowEventName(String);

impl WorkflowEventName {
    fn parse(value: &str) -> Result<Self, WorkflowReplayError> {
        match value {
            DISCOVERY_EVIDENCE_RECORDED | PRODUCT_HYPOTHESIS_FORMED => Ok(Self(value.to_owned())),
            _ => Err(WorkflowReplayError("event_unknown")),
        }
    }

    fn as_str(&self) -> &str {
        &self.0
    }
}

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

impl Default for WorkflowProjection {
    fn default() -> Self {
        Self::initial()
    }
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
            1 => WorkflowDecision {
                frontier: 1,
                next: "FormProductHypothesis",
                evidence: "derived",
                events: "ProductHypothesisFormed",
            },
            _ => unreachable!("a workflow projection only exposes known frontiers"),
        }
    }

    pub fn apply(self, event: &WorkflowEvent) -> Self {
        self.try_apply(event).unwrap_or(self)
    }

    pub fn try_apply(self, event: &WorkflowEvent) -> Result<Self, WorkflowReplayError> {
        match event {
            WorkflowEvent::DiscoveryEvidenceRecordedV1 {
                command,
                event,
                observation,
                ..
            } => {
                let command = WorkflowCommandName::parse(command)?;
                let event = WorkflowEventName::parse(event)?;
                if observation.trim().is_empty() || observation.len() > 1024 {
                    return Err(WorkflowReplayError("invalid_observation"));
                }
                match command.as_str() {
                    RECORD_DISCOVERY_EVIDENCE => {
                        if event.as_str() != DISCOVERY_EVIDENCE_RECORDED {
                            return Err(WorkflowReplayError("event_not_emitted_by_command"));
                        }
                        if self.frontier != 0 {
                            return Err(WorkflowReplayError("command_not_eligible"));
                        }
                        Ok(Self { frontier: 1 })
                    }
                    FORM_PRODUCT_HYPOTHESIS => {
                        if event.as_str() != PRODUCT_HYPOTHESIS_FORMED {
                            return Err(WorkflowReplayError("event_not_emitted_by_command"));
                        }
                        Err(WorkflowReplayError("command_not_eligible"))
                    }
                    _ => Err(WorkflowReplayError("event_unknown")),
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use eventcore::StreamId;

    use super::{
        WorkflowEvent, WorkflowProjection, FORM_PRODUCT_HYPOTHESIS, PRODUCT_HYPOTHESIS_FORMED,
        RECORD_DISCOVERY_EVIDENCE, WORKFLOW_STREAM,
    };

    #[test]
    fn initial_projection_selects_only_discovery_evidence_recording() {
        assert_eq!(
            WorkflowProjection::initial().decision().next,
            "RecordDiscoveryEvidence"
        );
    }

    #[test]
    fn unknown_semantic_event_does_not_advance_the_projection() {
        let event = WorkflowEvent::DiscoveryEvidenceRecordedV1 {
            workflow_id: StreamId::try_new(WORKFLOW_STREAM).expect("valid stream"),
            command: RECORD_DISCOVERY_EVIDENCE.to_owned(),
            event: "UnknownWorkflowEvent".to_owned(),
            observation: "fixture".to_owned(),
        };

        assert_eq!(
            WorkflowProjection::initial()
                .apply(&event)
                .decision()
                .frontier,
            0
        );
    }

    #[test]
    fn ineligible_semantic_command_does_not_advance_the_projection() {
        let event = WorkflowEvent::DiscoveryEvidenceRecordedV1 {
            workflow_id: StreamId::try_new(WORKFLOW_STREAM).expect("valid stream"),
            command: FORM_PRODUCT_HYPOTHESIS.to_owned(),
            event: PRODUCT_HYPOTHESIS_FORMED.to_owned(),
            observation: "fixture".to_owned(),
        };

        assert_eq!(
            WorkflowProjection::initial()
                .apply(&event)
                .decision()
                .frontier,
            0
        );
    }
}
use eventcore::{
    CommandError, CommandLogic, CommandStreams, Event, NewEvents, StreamDeclarations, StreamId,
};
use serde::{Deserialize, Serialize};

pub const WORKFLOW_STREAM: &str = "workflow-local";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum WorkflowEvent {
    DiscoveryEvidenceRecordedV1 {
        workflow_id: StreamId,
        command: String,
        event: String,
        observation: String,
    },
}

impl Event for WorkflowEvent {
    fn stream_id(&self) -> &StreamId {
        match self {
            Self::DiscoveryEvidenceRecordedV1 { workflow_id, .. } => workflow_id,
        }
    }
    fn event_type_name() -> &'static str {
        "TiberWorkflowEvent"
    }
}

pub struct RecordDiscoveryEvidence {
    pub workflow_id: StreamId,
    pub observation: String,
}

pub struct RecordWorkflowTestEvent {
    pub workflow_id: StreamId,
    pub command: String,
    pub event: String,
    pub observation: String,
}

impl CommandStreams for RecordDiscoveryEvidence {
    fn stream_declarations(&self) -> StreamDeclarations {
        StreamDeclarations::try_from_streams(vec![self.workflow_id.clone()])
            .expect("valid workflow stream")
    }
}

impl CommandLogic for RecordDiscoveryEvidence {
    type Event = WorkflowEvent;
    type State = WorkflowProjection;
    fn apply(&self, state: WorkflowProjection, event: &WorkflowEvent) -> WorkflowProjection {
        state.apply(event)
    }
    fn handle(&self, state: WorkflowProjection) -> Result<NewEvents<WorkflowEvent>, CommandError> {
        if state.frontier != 0 {
            return Err(CommandError::from("stale frontier"));
        }
        Ok(vec![WorkflowEvent::DiscoveryEvidenceRecordedV1 {
            workflow_id: self.workflow_id.clone(),
            command: RECORD_DISCOVERY_EVIDENCE.to_owned(),
            event: DISCOVERY_EVIDENCE_RECORDED.to_owned(),
            observation: self.observation.clone(),
        }]
        .into())
    }
}

impl CommandStreams for RecordWorkflowTestEvent {
    fn stream_declarations(&self) -> StreamDeclarations {
        StreamDeclarations::try_from_streams(vec![self.workflow_id.clone()])
            .expect("valid workflow stream")
    }
}

impl CommandLogic for RecordWorkflowTestEvent {
    type Event = WorkflowEvent;
    type State = WorkflowProjection;

    fn apply(&self, state: WorkflowProjection, event: &WorkflowEvent) -> WorkflowProjection {
        state.apply(event)
    }

    fn handle(&self, _state: WorkflowProjection) -> Result<NewEvents<WorkflowEvent>, CommandError> {
        Ok(vec![WorkflowEvent::DiscoveryEvidenceRecordedV1 {
            workflow_id: self.workflow_id.clone(),
            command: self.command.clone(),
            event: self.event.clone(),
            observation: self.observation.clone(),
        }]
        .into())
    }
}
