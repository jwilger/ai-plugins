---
name: development-workflow
description: Use for repository changes, debugging, review feedback, verification, final review, CI recovery, or deciding the next development lifecycle step.
---

# Development workflow

Read `.development-system.toml` before choosing a workflow. Treat it as the
project's single feature and delivery-policy source.

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

If pushed CI fails unexpectedly, stop unrelated work. When
`[features].beads = true`, pour or create one `ci-recovery` molecule, claim it
atomically, and acquire the Beads merge slot before repairing or rerunning it.
An intentional failure while testing an active `ci-workflow-slice` remains
related work in that slice rather than a separate incident. When Beads is
disabled, follow the same single-owner recovery discipline locally.

Load [workflow rules](references/workflow-rules.md) only when executing or
reviewing a change.
