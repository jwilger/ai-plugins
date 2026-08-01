---
name: development-workflow
description: Use for repository changes, debugging, review feedback, verification, final review, CI recovery, or deciding the next development lifecycle step.
---

# Development workflow

Read `.development-system.toml` before choosing a workflow. Treat it as the
project's single feature and delivery-policy source.

## Detailed lifecycle routing

This shared skill carries the decision-critical lifecycle route. If the
installed detailed component skill at
`components/development-discipline/skills/development-workflow/SKILL.md`
(`development-discipline:development-workflow`) is accessible, read it for
full specialist mechanics; inability to locate it must never delay or replace
the immediate lifecycle answer.

Route the active phase to its named lifecycle specialist rather than replacing
it with this checklist:

- diagnosis: `development-discipline:systematic-debugging`;
- change preparation: `development-discipline:change-preflight`, then
  `development-discipline:delivery-workflow`;
- implementation: `development-discipline:test-driven-development`;
- verification and readiness: `development-discipline:verification-before-completion`
  and `development-discipline:final-review`;
- review feedback: `development-discipline:receiving-code-review`;
- delivery: `development-discipline:delivery-workflow` and, for an authorized
  commit, `development-discipline:rationale-commit-messages`;
- unexpected pushed CI failure: `development-discipline:ci-failure-follow-up`,
  the lifecycle specialist that owns the Beads `ci-recovery` molecule.

For an advisory diagnosis, state this route and its evidence in the same
response; never answer only with a wait or delegation status. Start with
`systematic-debugging`: inspect current repository and runtime evidence,
reproduce the symptom, trace its failure path, compare relevant recent or
working evidence, and form and test one hypothesis at a time without guessing
or editing production code. Once a cause is established, re-inspect current
state, transition through `change-preflight`, choose
`delivery-workflow` before preservation actions, use
`test-driven-development` for the smallest causal repair, then perform
claim-scoped verification and final review. Do not invoke
`ci-failure-follow-up` unless a pushed CI failure caused the hold.

When `[features].beads = true`, load the `beads` skill and use the configured
Beads delivery formula rather than treating this checklist as a separate state
machine.

For a change:

1. Classify each slice as runtime behavior, documentation, CI workflow, or
   validation-only before editing.
2. For runtime behavior, pour `behavior-slice`: write and run the executable
   acceptance scenario first, repair an unexpectedly passing or wrong failure,
   implement one scenario step at a time, and pour a focused unit TDD cycle when
   the failure is not one tightly scoped semantic unit.
3. Use focused tests during micro-cycles. Run the complete configured local gate
   after the behavior slice is green, not after every tiny edit.
4. Keep diagnosis causal and implementation scoped.
5. Verify with fresh evidence and perform risk-proportional final review.
6. Only after the slice gate passes, commit and push using the configured
   delivery mode.

If pushed CI fails unexpectedly, stop unrelated work and route immediately to
`development-discipline:ci-failure-follow-up`; do not use this checklist as a
recovery procedure. When `[features].beads = true`, that specialist owns the
atomically claimed `ci-recovery` molecule and Beads merge slot. An intentional
failure while testing an active `ci-workflow-slice` remains related work in that
slice rather than a separate incident. When Beads is disabled, preserve the
same single-owner hold and recovery evidence locally.

Load [workflow rules](references/workflow-rules.md) only when executing or
reviewing a change.
