---
name: development-workflow
description: Use when a development request needs lifecycle routing for diagnosis, implementation, review feedback, PR or MR creation, readiness, CI failure, verification, or delivery.
---

# Development workflow

Route the current phase; do not replace specialist instructions with another
end-to-end procedure.

## Establish context first

Before routing, read the current user direction, repository-local instructions,
and current repository, task, branch or worktree, review, and CI state that can
change which phase is active. Apply them in that order. Do not infer a pull
request, delivery mode, approval, or completed gate that the evidence does not
establish.

Choose only the smallest set of specialists needed for the current phase. When
one phase completes, inspect state again before selecting the next one.
Do not preselect a specialist for a failure or branch that has not occurred;
route that future phase only if current state activates it.

When delegating any selected specialist, apply the canonical `model-routing`
matrix to that task. Lifecycle routing selects what work is needed;
`model-routing` independently selects the eligible task-local model and its
verification boundary.

## Mechanical transitions are not authorization

When the Development Discipline MCP is installed, its `workflow.start`,
`workflow.record_red`, `workflow.authorize_implementation`,
`workflow.record_green`, `workflow.authorize_review`,
`workflow.record_clean_review`, and `workflow.authorize_delivery` calls are
mechanical lifecycle transitions. They prove that required predecessor evidence
was recorded and control which repository mutation phase is open. Despite the
`authorize_*` method names, they do not grant human or policy authorization for
an edit, destructive operation, commit, push, PR/MR, merge, release, or other
externally visible action.

Derive action authorization separately from current user direction and
repository policy. Require both conditions before acting: the mechanical phase
permits the operation, and the user or repository authorizes that exact class of
operation. A model choice, successful transition, review recommendation, or
standing instruction for a different operation satisfies neither condition.

When the user asks only for a workflow explanation, describe the inspection and
routing that would occur without claiming to have performed it. For an answer
or domain-review request that is not a final review of completed development
work, explicitly skip implementation and delivery specialists unless the user
separately requests a repository change. A request to review a completed diff
still routes to `final-review`, even when that review itself is read-only.

## Routing table

| Current phase                                  | Route to                                                                                   | Continue when                                                                                                       |
| ---------------------------------------------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| Answer or review only                          | The relevant domain, documentation, security, OpenAI, or browser capability                | The requested answer or review is complete; do not enter implementation without a change request                    |
| Diagnose unexpected behavior                   | `systematic-debugging`                                                                     | Evidence identifies the cause; implementation is separately requested or already in scope                           |
| Prepare a substantive change                   | `change-preflight`, then `delivery-workflow` to select delivery mode and increment cadence | Every required surface has an evidence-backed decision and the delivery policy is known before preservation actions |
| Implement a feature, fix, removal, or refactor | `test-driven-development`, then the selected `delivery-workflow` checkpoint                | RED is used when applicable; the increment is tested, lightly reviewed, gated, and delivered                        |
| Verify a completion claim                      | `verification-before-completion`                                                           | Fresh evidence covers the exact increment or terminal claim                                                         |
| Review the delivered completed change          | `final-review`                                                                             | All increments are delivered and the coordinator reports completion for that exact identity                         |
| Choose commit and publication mode             | `delivery-workflow`, plus `rationale-commit-messages` when a commit is authorized          | The increment checkpoint or post-review exact-identity readiness evidence is current                                |
| Create or update a PR or MR                    | `delivery-workflow` for authorization and mode, then the available forge capability        | The PR/MR exists at the intended exact head and its URL and state are recorded                                      |
| Respond to a pushed CI failure                 | `ci-failure-follow-up`                                                                     | A causal repair or unchanged-revision rerun reaches terminal success                                                |
| Respond to review feedback                     | `receiving-code-review`, then the applicable implementation and verification specialists   | Valid feedback is resolved or technically defended                                                                  |
| Monitor a PR or MR through readiness           | `babysit-pr` when available and the selected delivery mode actually uses a PR or MR        | Required checks, review, approval, and merge state are terminal for the exact head revision                         |

For ordinary implementation, the usual sequence is repository inspection,
`change-preflight`, early `delivery-workflow` selection, one
`test-driven-development` applicability decision and one increment at a time,
`verification-before-completion`, and the selected `delivery-workflow`
checkpoint for every increment. Only after every planned increment is delivered
does `final-review` inspect that exact identity; its clean result then returns to
`delivery-workflow` solely for exact-identity readiness, not to manufacture a
new commit or checkpoint.
`rationale-commit-messages` governs each authorized commit. The selected
delivery workflow governs whether work is committed, pushed directly, or sent
through a PR/MR, and whether exact-revision CI must reach a terminal result.

PR/MR creation is conditional: direct-to-trunk and local-only modes skip PR/MR
creation. In PR/MR mode, bind review, checks, approval, queue, and merge evidence
to the exact current head revision, and re-evaluate the entire readiness
decision whenever that head changes. Valid review-driven edits return through
the applicable implementation, verification, and final-review specialists.

Each named specialist owns its detailed mechanics, evidence, stop conditions,
and precedence rules. Do not copy those procedures into this router.

## Required phase boundaries

Keep these boundaries explicit in both action and advisory responses:

- Name every selected specialist and the boundary that activates it. Do not
  replace a required specialist name with generic wording such as "review it"
  or "use the normal workflow."

- Start with current user direction, repository instructions, and mutable state
  before describing any commit or push. For a substantive change, select
  `delivery-workflow` before the first preservation action. Deliver every green
  increment through that selected mode before terminal `final-review`. After a
  clean terminal review, recheck `delivery-workflow` only for readiness of the
  unchanged reviewed identity.
- For answer or domain-review work that is not a completed-diff final review,
  skip `change-preflight`,
  `test-driven-development`, `verification-before-completion`, `final-review`,
  `delivery-workflow`, `babysit-pr`, commits, pushes, PR/MR creation, and ticket
  creation unless a separate change request activates them.
- After diagnosis identifies the cause, inspect state again, then route through
  `change-preflight`, early `delivery-workflow`, `test-driven-development`,
  `verification-before-completion`, checkpoint delivery, and—only when all
  planned work is delivered—`final-review` for an authorized fix.
- Before creating or updating a PR/MR, inspect current repository state again,
  confirm PR/MR mode and authorization, select an available forge capability,
  and bind creation plus the recorded URL and state to the exact reviewed head.
  State this repository-state, delivery-mode, authorization, and
  forge-capability recheck explicitly.
- For an existing PR/MR, capture the exact head before evaluating feedback or
  readiness. Route valid feedback through implementation, verification, and
  final review; monitor checks, approval, queue, and merge state without
  wasteful polling; and re-evaluate everything if the head changes. Do not
  enqueue, enable auto-merge, approve, or merge without current authorization.
  A review-driven code change is not ready until `final-review` completes for
  the changed head.
- After a pushed CI failure, keep `ci-failure-follow-up` as the only active
  lifecycle specialist. Diagnosis or authorization alone does not release the
  hold: resume unrelated work only after its causal repair or unchanged-head
  rerun reaches terminal success for the exact revision.

## Capability-aware fallback

Inspect the capabilities actually available in the current harness before
invoking a specialist. Never claim to call an unavailable skill, agent, MCP
server, browser, forge integration, or documentation source.

Capability fallback does not change the active lifecycle phase. Unexpected
behavior remains routed to `systematic-debugging`; unavailable documentation or
tooling specialists change how evidence is obtained, not whether diagnosis is
required before a fix.

When a named specialist is unavailable, preserve its intended outcome with the
smallest allowed phase-equivalent fallback:

1. follow current user and repository instructions;
2. use repository-pinned evidence and available local tools first;
3. use an available approved primary source or forge interface when current
   external evidence is required;
4. state which specialist was unavailable, which fallback supplied the
   evidence, and any remaining limitation.

Do not install a replacement tool, weaken a required gate, or rely on stale
model memory merely to keep moving. If no available and authorized capability
can satisfy a required evidence, approval, security, or external-state gate,
stop at that gate and request the missing direction or state change.
