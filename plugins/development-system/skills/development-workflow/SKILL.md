---
name: development-workflow
description: Use for repository changes, debugging, review feedback, verification, final review, CI recovery, or deciding the next development lifecycle step.
---

# Development workflow

Read `.development-system.toml` before choosing a workflow. Treat it as the
project's single feature and delivery-policy source.

For a change:

1. Classify the change surfaces before editing.
2. Work test-first for behavior changes.
3. Keep diagnosis causal and implementation scoped.
4. Verify with fresh evidence.
5. Perform risk-proportional final review.
6. Deliver using the configured delivery mode.

If pushed CI fails unexpectedly, stop unrelated work. When
`[features].beads = true`, pour or create one `ci-recovery` molecule, claim it
atomically, and acquire the Beads merge slot before repairing or rerunning it.
An intentional failure while testing an active `ci-workflow-slice` remains
related work in that slice rather than a separate incident. When Beads is
disabled, follow the same single-owner recovery discipline locally.

Load [workflow rules](references/workflow-rules.md) only when executing or
reviewing a change.
