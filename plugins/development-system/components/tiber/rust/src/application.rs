use eventcore::{
    CommandError, CommandLogic, CommandStreams, NewEvents, StreamDeclarations, StreamId,
};

use crate::domain::{apply_ticket_state, TicketEvent, TicketState};

pub struct PrioritizeTicket {
    pub ticket_id: StreamId,
    pub board_id: StreamId,
    pub priority: u8,
}

impl CommandStreams for PrioritizeTicket {
    fn stream_declarations(&self) -> StreamDeclarations {
        StreamDeclarations::try_from_streams(vec![self.ticket_id.clone(), self.board_id.clone()])
            .expect("valid priority streams")
    }
}

impl CommandLogic for PrioritizeTicket {
    type Event = TicketEvent;
    type State = TicketState;

    fn apply(&self, state: TicketState, event: &TicketEvent) -> TicketState {
        apply_ticket_state(state, &self.ticket_id, event)
    }

    fn handle(&self, state: TicketState) -> Result<NewEvents<TicketEvent>, CommandError> {
        if !state.exists {
            return Err(CommandError::from("ticket does not exist"));
        }
        Ok(vec![TicketEvent::BoardTicketPrioritySetV1 {
            board_id: self.board_id.clone(),
            ticket_id: self.ticket_id.clone(),
            priority: self.priority,
        }]
        .into())
    }
}

pub struct CompleteTicket {
    pub ticket_id: StreamId,
    pub board_id: StreamId,
    pub owner: String,
}

impl CommandStreams for CompleteTicket {
    fn stream_declarations(&self) -> StreamDeclarations {
        StreamDeclarations::try_from_streams(vec![self.ticket_id.clone(), self.board_id.clone()])
            .expect("valid completion streams")
    }
}

impl CommandLogic for CompleteTicket {
    type Event = TicketEvent;
    type State = TicketState;

    fn apply(&self, state: TicketState, event: &TicketEvent) -> TicketState {
        apply_ticket_state(state, &self.ticket_id, event)
    }

    fn handle(&self, state: TicketState) -> Result<NewEvents<TicketEvent>, CommandError> {
        if !state.exists {
            return Err(CommandError::from("ticket does not exist"));
        }
        if state.completed {
            return Err(CommandError::from("ticket is already completed"));
        }
        if state.owner.as_deref() != Some(self.owner.as_str()) {
            return Err(CommandError::from("ticket is not claimed by owner"));
        }
        Ok(vec![
            TicketEvent::TicketCompletedV1 {
                ticket_id: self.ticket_id.clone(),
                owner: self.owner.clone(),
            },
            TicketEvent::BoardTicketCompletedV1 {
                board_id: self.board_id.clone(),
                ticket_id: self.ticket_id.clone(),
                owner: self.owner.clone(),
            },
        ]
        .into())
    }
}
