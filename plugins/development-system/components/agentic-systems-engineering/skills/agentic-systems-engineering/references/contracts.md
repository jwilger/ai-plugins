# Prompt, Output, And Tool Contracts

Treat every model boundary as an API boundary.

## Prompt Contract

- State task, audience, input schema, allowed context sources, provenance and
  trust labels, forbidden assumptions, and abstention or escalation conditions.
- Distinguish developer instructions from retrieved or user-provided data.
- Keep examples representative and label them as examples, not hidden rules.
- Keep stable instructions ahead of volatile context to support caching when the
  provider offers it.

## Structured Output

- Parse model output into domain types at the boundary.
- Classify parse failure, schema-validation failure, domain-invariant failure,
  and policy or safety rejection separately. Give each a stable error shape and
  state whether it is retryable.
- Prefer explicit schemas over prose-only formatting requirements.
- Include a repair path only if the repair path itself is bounded and tested.

## Tool Use

- Define each tool's input and output schemas, authorization scope, side-effect
  class, error codes and retryability, timeout, retry/backoff limit, and
  approval gate.
- Give each call the minimum read, write, and data scope needed for the step.
- Retry side-effecting calls only with a stable idempotency key or equivalent
  deduplication token. Reconcile an unknown outcome before another write.
- Treat tool outputs and third-party or dynamically discovered tool descriptions
  as untrusted inputs.
- Put human approval before irreversible actions, not after them.
