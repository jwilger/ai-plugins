use eventcore::{
    CommandError, CommandLogic, CommandStreams, NewEvents, StreamDeclarations, StreamId,
};

use crate::domain::{apply_claim_state, ClaimState, TicketEvent};

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
    type State = ClaimState;

    fn apply(&self, state: ClaimState, event: &TicketEvent) -> ClaimState {
        apply_claim_state(state, &self.ticket_id, event)
    }

    fn handle(&self, state: ClaimState) -> Result<NewEvents<TicketEvent>, CommandError> {
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
