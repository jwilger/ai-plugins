# Agent Loops, Orchestration, And Durability

Agentic systems need explicit control surfaces because the model is not the
control plane.

## Loop Bounds

- Set relevant maxima for model turns, tool calls, retries by error class,
  elapsed time, tokens, and spend.
- Terminate on success, terminal error, budget exhaustion, cancellation, or
  no-progress detection. Record the typed termination reason.
- Detect no progress when the same action and failure recur or durable task state
  does not change. Checkpoint and escalate instead of looping.

## Orchestration

- Use one agent unless work has independently verifiable outputs, a distinct
  context or authority boundary, or a measured parallel-latency benefit.
- Add specialized agents only when that separation improves reliability,
  authority boundaries, or observability enough to justify cost.
- Make handoff contracts explicit: state, artifact, authority, expected output,
  and fallback path.

## Durability

- Checkpoint before and after meaningful side effects.
- Use stable idempotency keys or equivalent deduplication tokens for retried
  external writes, and reconcile calls whose outcome is unknown.
- Resume from committed state, not from model memory.
- Put human approval and commit gates before irreversible effects.
