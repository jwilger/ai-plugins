use eventcore::{Event, StreamId};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum TicketEvent {
    TicketCreatedV1 {
        ticket_id: StreamId,
        title: String,
    },
    TicketClaimedV1 {
        ticket_id: StreamId,
        owner: String,
    },
    TicketClaimReleasedV1 {
        ticket_id: StreamId,
        owner: String,
    },
    TicketCompletedV1 {
        ticket_id: StreamId,
        owner: String,
    },
    BoardTicketClaimedV1 {
        board_id: StreamId,
        ticket_id: StreamId,
        owner: String,
    },
    BoardTicketClaimReleasedV1 {
        board_id: StreamId,
        ticket_id: StreamId,
        owner: String,
    },
    BoardTicketCompletedV1 {
        board_id: StreamId,
        ticket_id: StreamId,
        owner: String,
    },
    BoardTicketPrioritySetV1 {
        board_id: StreamId,
        ticket_id: StreamId,
        priority: u8,
    },
    BoardTicketDependencyAddedV1 {
        board_id: StreamId,
        ticket_id: StreamId,
        dependency_id: StreamId,
    },
    BoardTicketDependencyRemovedV1 {
        board_id: StreamId,
        ticket_id: StreamId,
        dependency_id: StreamId,
    },
}

impl Event for TicketEvent {
    fn stream_id(&self) -> &StreamId {
        match self {
            Self::TicketCreatedV1 { ticket_id, .. }
            | Self::TicketClaimedV1 { ticket_id, .. }
            | Self::TicketClaimReleasedV1 { ticket_id, .. }
            | Self::TicketCompletedV1 { ticket_id, .. } => ticket_id,
            Self::BoardTicketClaimedV1 { board_id, .. }
            | Self::BoardTicketClaimReleasedV1 { board_id, .. }
            | Self::BoardTicketCompletedV1 { board_id, .. }
            | Self::BoardTicketPrioritySetV1 { board_id, .. }
            | Self::BoardTicketDependencyAddedV1 { board_id, .. }
            | Self::BoardTicketDependencyRemovedV1 { board_id, .. } => board_id,
        }
    }

    fn event_type_name() -> &'static str {
        "TiberTicketEvent"
    }
}

#[derive(Default)]
pub struct BoardDependencyState {
    pub tickets: BTreeSet<String>,
    pub completed: BTreeSet<String>,
    dependencies: BTreeMap<String, BTreeSet<String>>,
}

impl BoardDependencyState {
    pub fn unresolved_dependencies(&self, ticket_id: &str) -> Vec<String> {
        self.dependencies
            .get(ticket_id)
            .into_iter()
            .flatten()
            .filter(|dependency_id| !self.completed.contains(*dependency_id))
            .cloned()
            .collect()
    }

    pub fn has_dependency(&self, ticket_id: &str, dependency_id: &str) -> bool {
        self.dependencies
            .get(ticket_id)
            .is_some_and(|dependencies| dependencies.contains(dependency_id))
    }

    pub fn would_create_cycle(&self, ticket_id: &str, dependency_id: &str) -> bool {
        let mut visited = BTreeSet::new();
        self.reaches(dependency_id, ticket_id, &mut visited)
    }

    fn reaches(&self, from: &str, target: &str, visited: &mut BTreeSet<String>) -> bool {
        if from == target {
            return true;
        }
        if !visited.insert(from.to_owned()) {
            return false;
        }
        self.dependencies.get(from).is_some_and(|dependencies| {
            dependencies
                .iter()
                .any(|dependency| self.reaches(dependency, target, visited))
        })
    }
}

pub fn apply_board_dependency_state(
    mut state: BoardDependencyState,
    event: &TicketEvent,
) -> BoardDependencyState {
    match event {
        TicketEvent::TicketCreatedV1 { ticket_id, .. } => {
            state.tickets.insert(ticket_id.to_string());
        }
        TicketEvent::TicketCompletedV1 { ticket_id, .. }
        | TicketEvent::BoardTicketCompletedV1 { ticket_id, .. } => {
            state.completed.insert(ticket_id.to_string());
        }
        TicketEvent::BoardTicketDependencyAddedV1 {
            ticket_id,
            dependency_id,
            ..
        } => {
            state
                .dependencies
                .entry(ticket_id.to_string())
                .or_default()
                .insert(dependency_id.to_string());
        }
        TicketEvent::BoardTicketDependencyRemovedV1 {
            ticket_id,
            dependency_id,
            ..
        } => {
            if let Some(dependencies) = state.dependencies.get_mut(ticket_id.as_str()) {
                dependencies.remove(dependency_id.as_str());
            }
        }
        _ => {}
    }
    state
}

#[derive(Default)]
pub struct TicketState {
    pub exists: bool,
    pub owner: Option<String>,
    pub completed: bool,
}

pub fn apply_ticket_state(
    mut state: TicketState,
    ticket_id: &StreamId,
    event: &TicketEvent,
) -> TicketState {
    match event {
        TicketEvent::TicketCreatedV1 {
            ticket_id: event_ticket_id,
            ..
        } if event_ticket_id == ticket_id => state.exists = true,
        TicketEvent::TicketClaimedV1 {
            ticket_id: event_ticket_id,
            owner,
        } if event_ticket_id == ticket_id && !state.completed => state.owner = Some(owner.clone()),
        TicketEvent::TicketClaimReleasedV1 {
            ticket_id: event_ticket_id,
            owner,
        } if event_ticket_id == ticket_id && state.owner.as_deref() == Some(owner) => {
            state.owner = None
        }
        TicketEvent::TicketCompletedV1 {
            ticket_id: event_ticket_id,
            owner,
        } if event_ticket_id == ticket_id && state.owner.as_deref() == Some(owner) => {
            state.owner = None;
            state.completed = true;
        }
        _ => {}
    }
    state
}
