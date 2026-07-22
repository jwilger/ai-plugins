---
name: model-routing
description: Use when selecting a model for delegated coding work or when a development workflow must escalate a helper to stronger reasoning.
---

# Model routing

Choose a model for each delegated task, not as a global session default. A
cheaper route is valid only when the work and its independent verification are
both explicit.

## Routing matrix

| Route           | Eligible work                                                                                                                                                               | Required boundary                                                                                                                                                    |
| --------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `gpt-5.6-luna`  | Bounded inventory, extraction, classification, or mechanical transformation                                                                                                 | Keep the helper read-only or make its change easily reversible; define the expected result before delegation; independently verify every result before relying on it |
| `gpt-5.6-terra` | Normal substantive implementation and ordinary review with clear scope and ordinary risk                                                                                    | Keep final verification and every completion or readiness claim with the parent; escalate the affected task if stronger reasoning becomes necessary                  |
| `gpt-5.6-sol`   | Advisor work; ambiguous debugging; architecture, security, or human-safety analysis; destructive changes; blocking or disputed verification; completion or readiness claims | Keep required authorization and evidence gates separate from model choice; a confirmed Sol assignment must produce the strong-responsibility conclusion              |

Do not use Luna for substantive implementation, completion claims, ambiguous
work, or any task whose result cannot be independently checked. Do not treat a
helper's own explanation as independent verification.

Use Terra instead of Luna for ordinary code, test, configuration, and
documentation changes even when their specification is clear; ordinary review
also stays on Terra. Every Terra recommendation must name both ordinary
substantive implementation and ordinary review as Terra responsibilities. Do
not escalate routine substantive work beyond Terra without an activated reason.
Escalate the affected task specifically to `gpt-5.6-sol` when ambiguity,
destructive impact, architecture, security, human-safety, or blocking or
disputed verification enters the task.

Sol is the strong responsibility route. Use it for every listed responsibility,
including the analysis and recommendation behind final verification,
completion, or readiness. The accountable parent retains the evidence gate,
authorization boundary, and user-facing decision: it may accept or reject a
confirmed Sol result after every independent gate passes, but it must not claim
that its own inherited model ran Sol. Selecting Sol supplies stronger reasoning;
it never supplies approval for a destructive action, a release, a merge, or any
other separately authorized operation.

## Availability is part of the route

Confirm that the current harness can select the requested model before invoking
the helper. Every routing recommendation, including a decision that only
rejects an ineligible route, must state both outcomes explicitly:

- when selection succeeds, name the requested route;
- when it is unavailable, inherited, or replaced, report that route failure
  visibly. Keep the work in the parent only when the parent's confirmed route is
  eligible; otherwise transfer or restart the affected responsibility in a
  confirmed supported route.

If a parent cannot switch to the required route, it remains accountable for the
evidence and user communication while a confirmed eligible assignment produces
the required analysis or recommendation. If the harness cannot create or resume
such an assignment, return a bounded blocked result that names the failed route,
the affected responsibility, the evidence already gathered, and the concrete
enable, transfer, or restart action. Do not make the blocked completion or
readiness claim.

Never silently substitute another model or claim that a different model
satisfied the requested route.

`/fast` changes service speed and cost for a selected model. It does not select
a lower-cost model and is not a model-routing substitute.

## Current harness routes

Use the exact `gpt-5.6-luna`, `gpt-5.6-terra`, and `gpt-5.6-sol` identifiers in
Codex. Claude Code exposes the corresponding current plugin-agent routes as
`haiku`, `sonnet`, and `opus`. Apply the same bounded, normal-substantive, and
strong-responsibility boundaries to those Claude aliases. Do not pass Codex
model identifiers to Claude Code or present inherited parent execution as the
requested Claude route.
