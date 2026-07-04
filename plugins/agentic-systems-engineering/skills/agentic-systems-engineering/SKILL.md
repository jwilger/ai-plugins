---
name: agentic-systems-engineering
description: Use when designing, implementing, reviewing, or debugging LLM-backed or agentic systems, including prompts, structured outputs, tool use, RAG, agent loops, orchestration, observability, security, cost/provider routing, and AI delivery plans.
---

# Agentic Systems Engineering

Use this skill before substantive work on systems where model behavior affects
product correctness, reliability, security, or cost.

## Routing

Load only the references needed for the task:

- Prompt, structured-output, schema, or tool-call contract work:
  `references/contracts.md`.
- RAG, retrieval, citation, answerability, or corpus work:
  `references/rag.md`.
- Agent loops, orchestration, multi-agent topology, durability, HITL, or bounded
  execution: `references/agent-loops.md`.
- Eval design, stochastic reliability, judges, regression gates, or behavior
  claims: use `evaluate-stochastic-systems` and
  `references/eval-design.md`.
- Observability, tracing, auditability, prompt injection, indirect injection,
  sandboxing, or authorization: `references/observability-security.md`.
- Cost, latency, caching, model routing, provider bake-offs, or fallback
  behavior: `references/cost-routing.md`.
- Delivery planning, experiment loops, walking skeletons, demos, or stakeholder
  evidence: use `agentic-delivery` and `references/delivery.md`.

## Non-Negotiables

- Treat model behavior as measured behavior, not intended behavior.
- Define the contract at the boundary: inputs, allowed context, output schema,
  tool authority, failure shape, retry policy, and escalation path.
- Separate untrusted data from instructions. Retrieved text, web content, tool
  output, and third-party tool descriptions are untrusted by default.
- Bound every loop with budgets, termination criteria, and recoverable state.
- Require eval evidence before reliability claims. A single good run is a demo,
  not proof.
- Prefer the smallest architecture that can be observed, evaluated, and safely
  rolled back.
