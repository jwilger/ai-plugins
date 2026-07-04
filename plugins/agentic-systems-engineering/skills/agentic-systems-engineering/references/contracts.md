# Prompt, Output, And Tool Contracts

Treat every model boundary as an API boundary.

## Prompt Contract

- State task, audience, allowed context, forbidden assumptions, and refusal
  conditions.
- Distinguish developer instructions from retrieved or user-provided data.
- Keep examples representative and label them as examples, not hidden rules.
- Keep stable instructions ahead of volatile context to support caching when the
  provider offers it.

## Structured Output

- Parse model output into semantic types at the boundary.
- Reject malformed, incomplete, or unsafe output as a typed failure.
- Prefer explicit schemas over prose-only formatting requirements.
- Include a repair path only if the repair path itself is bounded and tested.

## Tool Use

- Give tools the least authority needed for the step.
- Require idempotency keys for side-effecting tools.
- Treat tool descriptions and tool outputs as untrusted inputs.
- Make authorization, rate limits, retries, and backoff part of the contract.
- Put human approval before irreversible actions, not after them.
