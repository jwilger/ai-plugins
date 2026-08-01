---
name: model-routing
description: Use when selecting a model for delegated coding work or when a development workflow must escalate a helper to stronger reasoning.
---

# Model routing

Choose a model for each delegated task, not as a global session default. A
cheaper route is valid only when the work and its independent verification are
both explicit.

Answer routing-classification questions directly; do not delegate the answer or
wait for another agent. Make every recommendation self-contained by naming the
task-local lower-tier identifier or strong-capability selection, the
corresponding Claude alias, the eligibility and exclusion rule, the capability
or accountability boundary, the escalation and unavailable-route behavior,
and that the global session default remains unchanged.

## Routing matrix

| Route                                | Eligible work                                                                                                                                                                                                                           | Required boundary                                                                                                                                                                   |
| ------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `gpt-5.6-luna` / `haiku`             | Bounded inventory, extraction, classification, or mechanical transformation                                                                                                                                                             | Keep the helper read-only or make its change easily reversible; define the expected result before delegation; independently verify every result before relying on it                |
| `gpt-5.6-terra` / `sonnet`           | Normal substantive implementation and ordinary review with clear scope and ordinary risk                                                                                                                                                | Return the substantive result to the accountable parent; route analysis behind final verification and every completion or readiness claim to a confirmed strong-responsibility role |
| Highest-capability eligible / `opus` | Advisor work; substantive human-consumable content; ambiguous debugging; architecture, security, or human-safety analysis; separately authorized destructive changes; blocking or disputed verification; completion or readiness claims | Keep required authorization and evidence gates separate from model choice; a confirmed strong assignment must produce the strong-responsibility result                              |

Do not use Luna for substantive implementation, completion claims, ambiguous
work, or any task whose result cannot be independently checked. Do not treat a
helper's own explanation as independent verification.

Use Terra instead of Luna for ordinary code, test, configuration, and
mechanical documentation changes even when their specification is clear;
ordinary review also stays on Terra. Substantive human-consumable content is
the explicit exception described below. Every Terra recommendation must name
both ordinary substantive implementation and ordinary review as Terra
responsibilities. Do not escalate other routine substantive work beyond Terra
without an activated reason.
Escalate the affected task specifically to the strong-responsibility route when
ambiguity, destructive impact, architecture, security, human-safety, or blocking
or disputed verification enters the task.

The highest-capability eligible model is the strong-responsibility route. Use a
confirmed strong assignment for every listed responsibility, including the
analysis and recommendation behind final verification, completion, or
readiness. A cheaper parent may coordinate evidence but may not substitute its
own reasoning for that confirmed strong result. A deterministic coordinator may
apply evidence and policy gates to produce the terminal review status; this is
not a model-routing assignment. The accountable parent retains the
authorization boundary and user-facing communication and may convey or reject
that status, but it must not make a contrary readiness claim from cheaper
inherited reasoning. Selecting the strong route supplies stronger reasoning; it
never supplies approval for a destructive action, a release, a merge, or any
other separately authorized operation.

Match capability to the responsibility: use the read-only `strong-reviewer` for
analysis, verification, approval recommendations, and readiness
recommendations. An assignment to decide whether a destructive operation should
be approved is analysis, so it stays with `strong-reviewer`. Use the writable
`strong-worker` for implementation that must itself remain on the strong route,
including non-destructive implementation with activated architecture, security,
human-safety, or ambiguity stakes. Destructive implementation additionally
requires its separate authorization gate to pass; non-destructive strong work
does not gain that gate merely from model selection. A review result does not
execute a mutation, and a Terra parent does not implicitly resume a strong
responsibility. Whenever a routing answer discusses a destructive operation,
name both sides of this boundary even if the immediate assignment covers only
approval or only execution.

Advisor is a strong responsibility. Dispatch it through its dedicated read-only
role with `xhigh` effort; do not lower its effort to the normal strong-role
default.

Substantive human-consumable content is also a strong responsibility. Route
prose, documentation, instructions, imagery, and UI/UX creation or substantial
revision through the public `content-authoring` skill. It must explicitly
dispatch the highest-capability eligible writable agent as `strong-worker` at
high effort, using the authoritative current-harness capability or upgrade
metadata described below. Status updates, ticket metadata, commit messages, and
mechanical summaries are excluded. The author may use an authorized specialized
image tool when the artifact requires one. Neither the writable model nor its
tools widen the task's existing file, publication, destructive-action, or other
authorization boundaries. If ranking, selection, high-effort writable launch,
or a required specialized tool cannot be confirmed, report a visible blocked
route and do not silently draft through a weaker fallback.

## Select the strong route

Inspect the eligible models advertised by the current harness and its
authoritative capability or upgrade metadata at dispatch time. Select the
highest-capability eligible model explicitly for the affected strong role.
Never infer capability from model names, lexical or list order, price, release
date, or an unverified claim of novelty.

For Codex, omit `model` from strong-role configuration and retain `high` effort.
Pass the capability-ranked model explicitly when spawning the role. Advisor's
own role retains `xhigh` effort. For Claude Code, use the moving `opus` alias
with `high` effort for normal strong roles; use Advisor's dedicated highest
supported effort where its role requires it. Treat the alias as a
harness-maintained capability route, not a version pin.

Keep read-only roles read-only and writable roles writable only within their
existing authorization boundary. Model or effort selection grants no additional
authority.

## Availability is part of the route

Confirm ranking, explicit selection, and launch before relying on a strong
assignment. Every routing recommendation, including a decision that only
rejects an ineligible route, must state both outcomes explicitly:

- when selection succeeds, name the selected route and confirmed role;
- when authoritative ranking, explicit selection, or launch is unavailable,
  report the route failure visibly and return a bounded blocked result.

Keep the work in the parent only when the parent's confirmed route is eligible;
otherwise transfer or restart the affected responsibility in a confirmed
supported route. If a parent cannot switch to the required route, it remains
accountable for evidence and user communication while a confirmed eligible
assignment produces the required analysis or recommendation.

If the harness cannot create or resume that assignment, make the blocked result
name the failed ranking, selection, or launch step; the affected responsibility;
the evidence already gathered; and the concrete enable, transfer, or restart
action. Do not make a blocked completion or readiness claim. Never silently
substitute another model or claim that an unconfirmed model satisfied the
strong route.

For Luna and Terra, confirm that the current harness can select the requested
exact route before invoking it. Apply the same visible-failure rule when either
route is unavailable, inherited, or replaced.

`/fast` changes execution speed for the selected model and may affect service
pricing. It does not select a lower-cost model, reduce the model-compute route,
or satisfy cost routing.

## Current harness routes

Use the exact `gpt-5.6-luna` and `gpt-5.6-terra` identifiers for the lower Codex
routes. Claude Code exposes the corresponding current plugin-agent routes as
`haiku` and `sonnet`, and exposes the moving `opus` alias for the strong route.
Apply the same bounded, normal-substantive, and strong-responsibility boundaries
to those Claude aliases. Do not pass Codex model identifiers to Claude Code or
present inherited parent execution as the requested route.
