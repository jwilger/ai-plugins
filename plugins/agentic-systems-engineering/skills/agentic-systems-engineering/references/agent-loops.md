# Agent Loops, Orchestration, And Durability

Agentic systems need explicit control surfaces because the model is not the
control plane.

## Loop Bounds

- Define a task budget: steps, time, tokens, cost, and retries.
- Define termination criteria before execution.
- Detect repeated failed actions and escalate rather than looping.
- Log why the loop stopped.

## Orchestration

- Use one agent until decomposition pressure is real.
- Add specialized agents only when isolation improves reliability, authority
  boundaries, or observability enough to justify cost.
- Make handoff contracts explicit: state, artifact, authority, expected output,
  and fallback path.

## Durability

- Checkpoint before and after meaningful side effects.
- Use idempotency keys for external writes.
- Resume from committed state, not from model memory.
- Put human approval and commit gates before irreversible effects.
