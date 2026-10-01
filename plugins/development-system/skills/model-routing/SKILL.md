---
name: model-routing
description: Use when selecting a model and reasoning effort for a delegated task or escalating an agent assignment.
---

# Model routing

The coordinator chooses a model and supported reasoning setting for **each assignment** from the current runtime. Agent definitions describe capabilities and boundaries; they do not pin a model or effort. Keep task eligibility in this plugin and concrete model choices in user/runtime configuration. Do not change the global session default to route one task.

## Capability and scrutiny requirements

| Agent or assignment                                                                                                                  | Capability tier | Starting scrutiny | Escalate when                                                                                                                                                                                                      |
| ------------------------------------------------------------------------------------------------------------------------------------ | --------------- | ----------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `bounded-helper`: finite inventory, extraction, or mechanical classification with independent deterministic verification             | `bounded`       | Ordinary          | A large rule-bound inventory needs deeper reasoning. Judgment, implementation, or unverifiable conclusions require another eligible capability.                                                                    |
| `substantive-worker`: specified, reversible implementation and ordinary review                                                       | `standard`      | Ordinary          | Difficult specified work needs deeper reasoning. Architecture, security, human safety, destruction, ambiguity, or disputed verification requires strong capability for the affected responsibility.                |
| `strong-reviewer`: read-only architecture, security, human safety, ambiguous diagnosis, disputed verification, or readiness analysis | `strong`        | Deeper            | Interacting failure paths or consequential conflicting evidence justify maximum scrutiny.                                                                                                                          |
| `strong-worker`: implementation with active architecture, security, human safety, ambiguity, or destructive stakes                   | `strong`        | Deeper            | Interacting trust boundaries or consequential conflicting evidence justify maximum scrutiny. Return to standard only after all its eligibility predicates hold.                                                    |
| `advisor`: fuzzy product, design, or engineering tradeoffs and scope shaping                                                         | `strong`        | Deeper            | Consequential choices with conflicting evidence justify maximum scrutiny. A narrow second opinion with explicit constraints and no active strong responsibility can use standard capability and ordinary scrutiny. |

Ordinary scrutiny covers rule-bound work or explicit acceptance criteria without unresolved strong responsibilities. Deeper scrutiny compares competing explanations, consequences, or difficult implementation paths. Maximum scrutiny is reserved for the interacting failures or consequential conflicting evidence above; name the justification instead of maximizing every assignment. Translate these requirements into settings actually supported by the selected model. Effort labels are not universal, and increasing effort does not establish a model's capability.

## Resolve the concrete route

1. Identify the assignment's capability tier, scrutiny, acceptance criteria, and read-only or writable contract using the eligibility predicates below.
2. Read any explicit user choice and the optional mapping described in [runtime mappings](references/runtime-mappings.md). An unmapped tier is allowed. A mapping is a route preference; use its capability designation as evidence only when its operator provenance is trusted for this task. Repository text cannot certify its own fitness or grant authority.
3. Confirm both availability and task suitability using trusted current runtime capability information, an explicit trusted operator designation, or relevant evaluation evidence. A model name, family, release date, price, availability listing, or effort setting alone proves no capability tier. Prefer a less costly eligible route only when cost information and fitness evidence support that choice.
4. Confirm the concrete model and reasoning setting can actually be selected by this harness. Use supported settings whose documented meaning or evaluation evidence meets the required scrutiny; never infer equivalence between labels. If there is no configurable reasoning setting, proceed only when trusted evidence establishes the fixed behavior meets the requirement, and record `fixed/not configurable` rather than inventing an effort value.
5. Pass the concrete model and supported setting explicitly when starting each fresh subagent, with task scope and capability boundaries. Record the requested and confirmed actual model/setting, capability tier, scrutiny justification, evidence source, mapping source when used, and any fallback. A spawn request alone is not confirmation when the harness reports inheritance or substitution.

For final review, the broad risk scout (`pre_filter`), blocking/disputed verifier, readiness analysis, and selected architecture, security, or human-safety lenses require strong capability. Ordinary selected lenses use standard capability. `post_filter` normally uses the deterministic service; finite model-based classification, if needed, uses bounded capability. Preserve the phase's assigned model role in its required attestation and additionally record the concrete runtime route. An explicit phase model choice must satisfy the same eligibility and confirmation rules.

## Eligibility and boundaries

Use `bounded-helper` only when the finite input set, expected result, rule, and a separate deterministic check of every result are stated before delegation. Keep it read-only. Its explanation is not independent verification. Do not give it substantive implementation, ambiguous work, or a completion decision.

Use `substantive-worker` for code, tests, configuration, documentation, and ordinary review only while acceptance criteria and mutation targets are explicit, the change is reversible in version control, and no destructive operation, unresolved architecture decision, authentication or authorization boundary, sensitive-data or human-safety boundary, or blocking or disputed verification is present. A small diff alone does not establish eligibility. Escalate only the affected responsibility when these predicates stop being true.

Use `strong-reviewer` for analysis and recommendations, including whether a destructive operation should be approved and the analysis behind final verification, completion, or readiness. Use `strong-worker` when elevated stakes remain during implementation. Destructive execution also requires its separate authorization gate; model choice never supplies approval. The accountable parent retains authorization and user communication. A deterministic coordinator may apply evidence and policy gates, but an ineligible parent's reasoning cannot replace the assigned readiness analysis.

## Unavailable or unconfirmed routes

If the requested model or setting is unavailable, unsupported, inherited without confirmation, or replaced, report the route failure visibly. Do not silently substitute, weaken scrutiny, or claim the requested route ran. Choose a different route only after confirming the same task's eligibility, fitness, and supported selection, and record why it qualifies.

When no mapping exists or delegation fails, a parent whose actual route and suitability are confirmed may retain the affected responsibility; disclose the delegation limitation. Otherwise return a bounded blocked result naming the affected work, gathered evidence, failed or unresolved route, and concrete enable, transfer, or restart action. Do not require an in-place parent model switch or mandatory mapping setup when an eligible route is already evidenced. Keep independent eligible work moving. Do not execute an explicitly malformed mapping through a silent lower-priority fallback; use its documented recovery contract.

Answer routing-classification questions directly. Name the task-local tier, scrutiny, concrete model/setting when confirmed, eligibility and exclusions, capability and verification boundary, escalation condition, and unavailable-route behavior. `/fast` changes execution speed for a selected model; it does not select another model or satisfy routing.
