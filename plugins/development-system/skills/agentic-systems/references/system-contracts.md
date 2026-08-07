# Agentic System Contracts

Use precise terms when they change the engineering action. Plain-language
equivalents are acceptable when they preserve the same contract.

## Instruction Contracts

Replace gate-changing adjectives with observable predicates. Name the trigger,
operation, success result, typed stop or fallback, and authorization boundary.
Select required evidence from the changed contract and repository gates; call
evidence insufficient when a required claim lacks fresh claim-mapped proof.
Evaluate the instruction with a natural positive case and close-boundary
negative cases for routing, evidence, stopping, and authorization. Grade the
operational consequence, not exact terminology.

## Model And Tool Boundaries

- Define input fields, allowed context sources, provenance and trust labels,
  output schema, domain invariants, refusal or escalation conditions, and the
  observable success criterion.
- Classify output failures separately: parse failure, schema-validation failure,
  domain-invariant failure, and policy or safety rejection. Give each failure a
  stable error shape and state whether it is retryable.
- For every tool, define input and output schemas, authorization scope,
  side-effect class, error codes and retryability, timeout, retry/backoff limit,
  and human-approval gate.
- Retry a side-effecting call only with a stable idempotency key or equivalent
  deduplication token. When the outcome is unknown, reconcile status before
  issuing another write.
- Treat user data, retrieved content, tool output, and third-party or dynamically
  discovered tool descriptions as untrusted data, not as instructions.

## Retrieval And Generation

- Measure retrieval coverage and ranking separately from answer quality, using
  task-appropriate metrics such as recall@k, precision@k, MRR, or nDCG.
- Evaluate answerability, groundedness or faithfulness to retrieved evidence,
  answer correctness, claim-to-source citation support, and abstention as
  separate stages.
- Apply authorization or ACL filters during retrieval. Define source freshness
  and precedence for stale or conflicting documents.
- Treat instructions embedded in retrieved content as indirect prompt injection.

## Agent Control

- Bound execution with maximum model turns, tool calls, retries by error class,
  elapsed time, tokens, and spend when each limit is relevant.
- Terminate on success, terminal error, budget exhaustion, cancellation, or
  no-progress detection. Record the typed termination reason.
- Detect no progress when the same action and failure recur or durable task state
  does not change. Checkpoint recoverable state and escalate instead of looping.
- Add another agent only for independently verifiable work, a distinct context or
  authority boundary, or a measured parallel-latency benefit. Define the handoff
  artifact, authority, expected output, and fallback.

## Observability, Security, And Routing

- Emit trace spans for model, retrieval, rerank, tool, policy, and handoff calls.
  Record model and prompt/template versions, routing or policy decision, token
  counts, latency, cost, and typed failure reason while redacting sensitive data.
- Derive high-impact actions from the deployment threat model, especially
  irreversible or externally visible writes, credential use, and privileged
  data access. Put approval before those actions.
- Route each step to the least-cost model that clears a predefined quality
  threshold and latency service-level objective. Test fallback behavior and do
  not silently route to a model below the required quality bar.
- Give semantic caches explicit keys, freshness limits, invalidation rules, and
  authorization boundaries. Optimize cost per successful task, not token price.
