use eventcore::{
    CommandError, CommandLogic, CommandStreams, NewEvents, StreamDeclarations, StreamId,
};

use crate::domain::{
    apply_board_dependency_state, apply_ticket_state, BoardDependencyState, TicketEvent,
    TicketState,
};

pub struct ClaimTicket {
    pub ticket_id: StreamId,
    pub board_id: StreamId,
    pub owner: String,
}

#[derive(Default)]
pub struct ClaimTicketState {
    ticket: TicketState,
    board: BoardDependencyState,
}

impl CommandStreams for ClaimTicket {
    fn stream_declarations(&self) -> StreamDeclarations {
        StreamDeclarations::try_from_streams(vec![self.ticket_id.clone(), self.board_id.clone()])
            .expect("valid claim streams")
    }
}

impl CommandLogic for ClaimTicket {
    type Event = TicketEvent;
    type State = ClaimTicketState;

    fn apply(&self, mut state: ClaimTicketState, event: &TicketEvent) -> ClaimTicketState {
        state.ticket = apply_ticket_state(state.ticket, &self.ticket_id, event);
        state.board = apply_board_dependency_state(state.board, event);
        state
    }

    fn handle(&self, state: ClaimTicketState) -> Result<NewEvents<TicketEvent>, CommandError> {
        if !state.ticket.exists {
            return Err(CommandError::from("ticket does not exist"));
        }
        if state.ticket.completed {
            return Err(CommandError::from("ticket is already completed"));
        }
        if state.ticket.owner.is_some() {
            return Err(CommandError::from("ticket already claimed"));
        }
        if !state
            .board
            .unresolved_dependencies(self.ticket_id.as_str())
            .is_empty()
        {
            return Err(CommandError::from("ticket has unresolved dependencies"));
        }
        Ok(vec![
            TicketEvent::TicketClaimedV1 {
                ticket_id: self.ticket_id.clone(),
                owner: self.owner.clone(),
            },
            TicketEvent::BoardTicketClaimedV1 {
                board_id: self.board_id.clone(),
                ticket_id: self.ticket_id.clone(),
                owner: self.owner.clone(),
            },
        ]
        .into())
    }
}

pub struct AddTicketDependency {
    pub ticket_id: StreamId,
    pub dependency_id: StreamId,
    pub board_id: StreamId,
}

pub struct RemoveTicketDependency {
    pub ticket_id: StreamId,
    pub dependency_id: StreamId,
    pub board_id: StreamId,
}

impl CommandStreams for RemoveTicketDependency {
    fn stream_declarations(&self) -> StreamDeclarations {
        StreamDeclarations::try_from_streams(vec![
            self.ticket_id.clone(),
            self.dependency_id.clone(),
            self.board_id.clone(),
        ])
        .expect("valid dependency streams")
    }
}

impl CommandLogic for RemoveTicketDependency {
    type Event = TicketEvent;
    type State = BoardDependencyState;

    fn apply(&self, state: BoardDependencyState, event: &TicketEvent) -> BoardDependencyState {
        apply_board_dependency_state(state, event)
    }

    fn handle(&self, state: BoardDependencyState) -> Result<NewEvents<TicketEvent>, CommandError> {
        let ticket_id = self.ticket_id.as_str();
        let dependency_id = self.dependency_id.as_str();
        if !state.tickets.contains(ticket_id) || !state.tickets.contains(dependency_id) {
            return Err(CommandError::from("ticket does not exist"));
        }
        if !state.has_dependency(ticket_id, dependency_id) {
            return Err(CommandError::from("dependency does not exist"));
        }
        Ok(vec![TicketEvent::BoardTicketDependencyRemovedV1 {
            board_id: self.board_id.clone(),
            ticket_id: self.ticket_id.clone(),
            dependency_id: self.dependency_id.clone(),
        }]
        .into())
    }
}

impl CommandStreams for AddTicketDependency {
    fn stream_declarations(&self) -> StreamDeclarations {
        StreamDeclarations::try_from_streams(vec![
            self.ticket_id.clone(),
            self.dependency_id.clone(),
            self.board_id.clone(),
        ])
        .expect("valid dependency streams")
    }
}

impl CommandLogic for AddTicketDependency {
    type Event = TicketEvent;
    type State = BoardDependencyState;

    fn apply(&self, state: BoardDependencyState, event: &TicketEvent) -> BoardDependencyState {
        apply_board_dependency_state(state, event)
    }

    fn handle(&self, state: BoardDependencyState) -> Result<NewEvents<TicketEvent>, CommandError> {
        let ticket_id = self.ticket_id.as_str();
        let dependency_id = self.dependency_id.as_str();
        if !state.tickets.contains(ticket_id) || !state.tickets.contains(dependency_id) {
            return Err(CommandError::from("ticket does not exist"));
        }
        if ticket_id == dependency_id {
            return Err(CommandError::from("ticket cannot depend on itself"));
        }
        if state.has_dependency(ticket_id, dependency_id) {
            return Err(CommandError::from("dependency already exists"));
        }
        if state.would_create_cycle(ticket_id, dependency_id) {
            return Err(CommandError::from("dependency would create a cycle"));
        }
        Ok(vec![TicketEvent::BoardTicketDependencyAddedV1 {
            board_id: self.board_id.clone(),
            ticket_id: self.ticket_id.clone(),
            dependency_id: self.dependency_id.clone(),
        }]
        .into())
    }
}

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
