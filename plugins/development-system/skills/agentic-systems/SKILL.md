---
name: agentic-systems
description: Use when designing, scaffolding, evaluating, or delivering LLM and agent systems, including eval harnesses, RAG, structured output, model tool use, orchestration, AI observability, provider comparison or routing, AI security, cost, and stochastic evaluation, when agentic_systems is enabled.
---

# Agentic systems

Require `[features].agentic_systems = true` in
`.development-system.toml`.

Before optimizing prompts or orchestration, define the boundary contract:
input schema, context sources and trust, output schema and domain invariants,
tool authority and side effects, typed failures and retryability, timeout and
idempotency policy, abstention or escalation conditions, and measurable
acceptance criteria.

Load only the reference needed for the task:

- Prompt, structured-output, tool, RAG, loop, observability, security, cost, or
  provider-routing work: `references/system-contracts.md`.
- Eval design, sampling, judges, regression gates, or behavior claims:
  `references/evaluation.md`.
- Creating a Promptfoo evaluation harness or production/ablation compositions:
  [scaffold agentic
  evals](../../components/agentic-systems-engineering/skills/scaffold-agentic-evals/SKILL.md).
- Repeated-run analysis or judge calibration: [evaluate stochastic
  systems](../../components/agentic-systems-engineering/skills/evaluate-stochastic-systems/SKILL.md).
- Shipping an agentic change with observable acceptance evidence: [agentic
  delivery](../../components/agentic-systems-engineering/skills/agentic-delivery/SKILL.md).

Use project-owned Promptfoo fixtures when configured. Prefer distinct cases for
population quality. Repeat the same case only for a named metric such as
per-input reliability, pass@k capability, pass^k reliability, or judge
variance. One successful run is not reliability evidence.

An eval scaffold must include representative nominal, boundary, expected-error
or refusal, partial-credit, adversarial, and regression cases; deterministic
guards for contract facts; and calibrated semantic judges only where exact
checks cannot express quality. Compare matched providers and models. Record the
exact provider, model, settings, prompt and tool versions, installed plugin
composition, distinct-case count, repeat count, named metric, aggregation rule,
and failure reason in repo-owned artifacts. Evaluate the production composition;
add a targeted-plugin ablation when attribution matters. CI may validate wiring
without credentials, while live provider runs stay in trusted execution.
