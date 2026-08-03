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
            | Self::TicketClaimReleasedV1 { ticket_id, .. } => ticket_id,
            Self::BoardTicketClaimedV1 { board_id, .. }
            | Self::BoardTicketClaimReleasedV1 { board_id, .. }
            | Self::BoardTicketPrioritySetV1 { board_id, .. } => board_id,
        }
    }

    fn event_type_name() -> &'static str {
        "TiberTicketEvent"
    }
}

#[derive(Default)]
pub struct ClaimState {
    pub exists: bool,
    pub owner: Option<String>,
}

pub fn apply_claim_state(
    mut state: ClaimState,
    ticket_id: &StreamId,
    event: &TicketEvent,
) -> ClaimState {
    match event {
        TicketEvent::TicketCreatedV1 {
            ticket_id: event_ticket_id,
            ..
        } if event_ticket_id == ticket_id => state.exists = true,
        TicketEvent::TicketClaimedV1 {
            ticket_id: event_ticket_id,
            owner,
        } if event_ticket_id == ticket_id => state.owner = Some(owner.clone()),
        TicketEvent::TicketClaimReleasedV1 {
            ticket_id: event_ticket_id,
            owner,
        } if event_ticket_id == ticket_id && state.owner.as_deref() == Some(owner) => {
            state.owner = None
        }
        _ => {}
    }
    state
}
