---
name: model-routing
description: Use when selecting a model and reasoning effort for a delegated task or escalating an agent assignment.
---

# Model routing

The coordinator chooses a model and reasoning effort for **each assignment**. Agent definitions describe capabilities and boundaries; they do not pin a model or effort. Do not change the global session default to route one task. Start with the defaults below, then use the task's evidence, stakes, and verification needs to choose a different route when warranted.

## Task-local defaults

| Agent or assignment                                                                                                                  | Default model | Default effort | Use a different route when                                                                                                                                                                                                                 |
| ------------------------------------------------------------------------------------------------------------------------------------ | ------------- | -------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `bounded-helper`: finite inventory, extraction, or mechanical classification with independent deterministic verification             | `gpt-6-luna`  | `low`          | Use Luna `medium` for a large but still rule-bound inventory. If judgment, implementation, or unverifiable conclusions appear, reassign to `substantive-worker` or `strong-reviewer` on an eligible route.                                 |
| `substantive-worker`: specified, reversible implementation and ordinary review with no unresolved trust or safety question           | `gpt-6-sol`   | `medium`       | Use Sol `high` for a difficult but well-specified change. Transfer an architecture, security, human-safety, destructive, ambiguous, or disputed-verification responsibility to a strong agent on Astra.                                    |
| `strong-reviewer`: read-only architecture, security, human-safety, ambiguous diagnosis, disputed verification, or readiness analysis | `gpt-6-astra` | `high`         | Use Astra `xhigh` for intertwined failure paths or a high-stakes decision with conflicting evidence. Sol `high` can handle a narrow, well-evidenced review once the strong uncertainty has been resolved.                                  |
| `strong-worker`: implementation whose architecture, security, human-safety, ambiguity, or destructive stakes remain active           | `gpt-6-astra` | `high`         | Use Astra `xhigh` for a complex migration with several interacting trust boundaries. Use Sol `high` only after no architecture, security, safety, ambiguity, or destructive stake remains active and the implementation is well specified. |
| `advisor`: fuzzy product, design, or engineering tradeoffs and scope shaping                                                         | `gpt-6-astra` | `high`         | Use Sol `medium` for a narrow second opinion with clear constraints; use Astra `xhigh` for consequential choices with several uncertain dependencies.                                                                                      |

For final review, default the broad risk scout (`pre_filter`) and the blocking or disputed verifier to the `strong-reviewer` route. Use the `substantive-worker` route for ordinary selected lenses, but the `strong-reviewer` route for selected architecture, security, or human-safety lenses. `post_filter` is normally deterministic; if a model assignment is actually needed for finite classification, use `bounded-helper`. The coordinator passes both the chosen model and effort when starting each fresh subagent and records the actual model role in its attestation.

## Eligibility and boundaries

Use `bounded-helper` only when the finite input set, expected result, rule, and a separate deterministic check of every result are stated before delegation. Keep it read-only. Its explanation is not independent verification. Do not give it substantive implementation, ambiguous work, or a completion decision.

Use `substantive-worker` for code, tests, configuration, documentation, and ordinary review only while acceptance criteria and mutation targets are explicit, the change is reversible in version control, and no destructive operation, unresolved architecture decision, authentication or authorization boundary, sensitive-data or human-safety boundary, or blocking or disputed verification is present. A small diff alone does not establish eligibility. Escalate only the affected responsibility when these predicates stop being true.

Use `strong-reviewer` for analysis and recommendations, including whether a destructive operation should be approved and the analysis behind final verification, completion, or readiness. Use `strong-worker` when elevated stakes remain during implementation. Destructive execution also requires its separate authorization gate; model choice never supplies approval. The accountable parent retains authorization and user communication. A deterministic coordinator may apply evidence and policy gates, but a cheaper parent's reasoning cannot replace the assigned readiness analysis.

## Spawn and availability contract

Before starting an agent, confirm that the current harness can select the chosen GPT-6 model **and** effort for that assignment. Pass both explicitly in the spawn call, together with task scope and the agent's read-only or writable contract. If the selected model or effort is unavailable, inherited, or replaced, report the route failure visibly. Choose another route only after checking the same task's eligibility; otherwise keep an eligible responsibility in the parent or return a bounded blocked result naming the affected work, gathered evidence, and the concrete enable, transfer, or restart action. Never claim an unconfirmed route was used.

Answer routing-classification questions directly. Name the task-local model and effort, eligibility and exclusions, capability and verification boundary, escalation condition, and unavailable-route behavior. `/fast` changes execution speed for a selected model; it does not select a cheaper model or satisfy routing.
