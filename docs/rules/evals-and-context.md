# Evals and context budget (skills, MCP, hooks)

**Effectiveness is evidence-driven, not vibes.** Choose evidence from the
release claim. Deterministic checks are sufficient for contracts they fully
decide, including retirement or deletion, exact file and routing state, schema,
format, metadata, and mechanical transformations. A skill, command, or MCP-tool
description edit does not automatically require a live provider eval.

Before running a provider eval, name the unresolved stochastic question and
record why deterministic verification cannot decide it. Use provider evidence
for model-dependent triggering, interpretation, semantic quality, routing,
tool-use, or reliability claims, with representative cases, declared metrics,
and variance analysis. Every retained provider case must carry that rationale.

Provider comparisons fail closed unless baseline and treatment contexts are
demonstrably isolated. A lift gate requires an explicit incremental-value
hypothesis. If the baseline unexpectedly succeeds, audit isolation, inherited
context, prompt leakage, and rubric specificity, then deliberately retire the
case or document why it measures absolute reliability instead. Never set
`valueGate.mode` to `none` merely to bypass a failed or surprising lift gate.

**Minimum-necessary context.** Skills, MCP schemas/descriptions, hooks, and any
injected context must use the **least context that stays effective** across Claude
Code, Codex, and future harnesses: progressive disclosure, triggers-only
descriptions, reference material loaded on demand. Measure each surface's token
footprint in both harnesses; reject regressions that don't buy proportional
effectiveness.
