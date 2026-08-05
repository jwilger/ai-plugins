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

If pushed CI fails, stop unrelated work. When `[features].tiber = true`,
establish one Tiber CI-recovery owner before repairing or rerunning it. When
Tiber is disabled, follow the same single-owner recovery discipline locally
without invoking Tiber.

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
