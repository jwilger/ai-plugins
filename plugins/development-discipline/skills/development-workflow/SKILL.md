---
name: development-workflow
description: Use when a development request needs routing from its current lifecycle phase to the smallest applicable specialist workflow.
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

## Routing table

| Current phase                         | Route to                                                                                   | Continue when                                                                                                       |
| ------------------------------------- | ------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| Answer or review only                 | The relevant domain, documentation, security, OpenAI, or browser capability                | The requested answer or review is complete; do not enter implementation without a change request                    |
| Diagnose unexpected behavior          | `systematic-debugging`                                                                     | Evidence identifies the cause; implementation is separately requested or already in scope                           |
| Prepare a substantive change          | `change-preflight`, then `delivery-workflow` to select delivery mode and increment cadence | Every required surface has an evidence-backed decision and the delivery policy is known before preservation actions |
| Implement a feature, fix, or refactor | `test-driven-development`                                                                  | The current behavior increment is green and its lightweight review is clean                                         |
| Verify a completion claim             | `verification-before-completion`                                                           | Fresh evidence covers the exact claim                                                                               |
| Review the completed change           | `final-review`                                                                             | The review coordinator reports completion for the current diff                                                      |
| Choose commit and publication mode    | `delivery-workflow`, plus `rationale-commit-messages` when a commit is authorized          | Repository-selected delivery evidence is current                                                                    |
| Respond to a pushed CI failure        | `ci-failure-follow-up`                                                                     | A causal repair or unchanged-revision rerun reaches terminal success                                                |
| Respond to review feedback            | `receiving-code-review`, then the applicable implementation and verification specialists   | Valid feedback is resolved or technically defended                                                                  |
| Monitor a PR or MR through readiness  | `babysit-pr` when available and the selected delivery mode actually uses a PR or MR        | Required checks, review, approval, and merge state are terminal for the exact head revision                         |

For ordinary implementation, the usual sequence is repository inspection,
`change-preflight`, early `delivery-workflow` selection, one
`test-driven-development` increment at a time,
`verification-before-completion`, `final-review`, and `delivery-workflow`.
`rationale-commit-messages` governs each authorized commit. The selected
delivery workflow governs whether work is committed, pushed directly, or sent
through a PR/MR, and whether exact-revision CI must reach a terminal result.

Each named specialist owns its detailed mechanics, evidence, stop conditions,
and precedence rules. Do not copy those procedures into this router.

## Capability-aware fallback

Inspect the capabilities actually available in the current harness before
invoking a specialist. Never claim to call an unavailable skill, agent, MCP
server, browser, forge integration, or documentation source.

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
