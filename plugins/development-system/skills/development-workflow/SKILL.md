---
name: development-workflow
description: Use when making repository changes, debugging, handling review feedback, verifying work, conducting final review, recovering CI, or deciding the next development lifecycle step.
---

# Development workflow

Read `.development-system.toml` before choosing a workflow. Treat it as the
project's single feature and delivery-policy source.

For a change:

1. Classify the change surfaces before editing.
2. Classify whether RED is required; work test-first only for new or changed
   first-party production behavior that has no applicable exemption.
3. Keep diagnosis causal and implementation scoped.
4. Verify with fresh evidence.
5. Perform risk-proportional final review.
6. Deliver using the configured delivery mode.

## Mechanical lifecycle gate

When the Development Discipline MCP is installed, repository mutations require
an active lifecycle. Start it before the first test, implementation, or
documentation edit:

1. Call `workflow.start` with `change_kind: "production"` for changed
   first-party behavior, or `change_kind: "exempt"` only for a documented RED
   exemption. An exemption skips only RED: record focused successful
   verification, complete final review, and authorize delivery normally.
2. For production work, add or update the focused test, then call
   `workflow.record_red` with the failing command and
   `workflow.authorize_implementation` before editing production code.
3. Run the focused passing test through `workflow.record_green`, then call
   `workflow.authorize_review`.
4. Complete the authoritative `final_review` state machine. Supply its
   completed returned `state` as `review_state` to
   `workflow.record_clean_review`, then call `workflow.authorize_delivery`.

The hook blocks unclassified mutations, production edits before RED, mutations
while review is required, post-review mutations before delivery authorization,
and all unrelated work during a shared CI-recovery hold.

If a lifecycle must be superseded before delivery, call `workflow.abandon`.
It preserves the terminal audit state and releases the repository for a new
lifecycle; never remove workflow-state files manually.

If pushed CI fails, stop unrelated work and establish one workflow-owned
CI-recovery owner through Development Discipline before repairing or rerunning
it. This gate is independent of whether the project uses Tiber for tickets.

Before starting a **new task**, inspect CI through the repository's forge. The
most recently completed build must be successful. A newer queued or running
build does not count as a replacement result, but any current build with a
completed failed job creates the CI-recovery hold immediately. This rule is
workflow-owned and applies whether or not the project uses Tiber.

For functionality removal, change production code first with tests untouched,
then run the suite and classify failures. Delete or update only expectations for
the removed behavior. Repair implementation whenever shared or retained
behavior fails, and never add a replacement test whose sole assertion is that
the removed capability is absent.

During review, apply the RED exemptions and audit existing surrounding tests,
not only changed lines. Report committed-text and workflow-structure assertions
for removal; vendor-example tests for removal or dependency-agnostic black-box
replacement; and overgrown developer utilities for extraction into a maintained
project or shipped subsystem while their valuable coverage is preserved. Retain
public black-box tests of user-visible behavior.

Load [workflow rules](references/workflow-rules.md) only when executing or
reviewing a change.
