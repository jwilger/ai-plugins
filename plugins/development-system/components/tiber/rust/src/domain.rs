use eventcore::{Event, StreamId};
use serde::{Deserialize, Serialize};

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
            | Self::BoardTicketPrioritySetV1 { board_id, .. } => board_id,
        }
    }

    fn event_type_name() -> &'static str {
        "TiberTicketEvent"
    }
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
