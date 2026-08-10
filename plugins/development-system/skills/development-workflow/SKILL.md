---
name: development-workflow
description: Use when making repository changes, debugging, handling review feedback, verifying work, conducting final review, recovering CI, or deciding the next development lifecycle step.
---

# Development workflow

Use `workspace-reader.status` before choosing a workflow. Outside Git, without
configuration, or with invalid configuration, inspection remains available and
project mutation is unavailable.

For every workflow question, load [workflow
rules](references/workflow-rules.md). Before editing, classify artifact impact
relations and RED applicability. Keep diagnosis causal, bind readiness claims
to post-mutation exact-revision evidence, select review lenses from changed
system boundaries, and use the configured delivery mode.

Load the retained specialist contract only when its intent matches:

- Change classification or preflight: [change
  preflight](../../components/development-discipline/skills/change-preflight/SKILL.md).
- Causal debugging: [systematic
  debugging](../../components/development-discipline/skills/systematic-debugging/SKILL.md).
- RED-GREEN-refactor behavior: [test-driven
  development](../../components/development-discipline/skills/test-driven-development/SKILL.md).
- Exact-revision evidence: [verification before
  completion](../../components/development-discipline/skills/verification-before-completion/SKILL.md).
- Final-review scope, findings, or verifier flow: [final
  review](../../components/development-discipline/skills/final-review/SKILL.md).
- Worker or reviewer selection: [model
  routing](../../components/development-discipline/skills/model-routing/SKILL.md).
- Failed pushed CI: [CI failure
  follow-up](../../components/development-discipline/skills/ci-failure-follow-up/SKILL.md).
- Lifecycle-aware delivery: [delivery
  workflow](../../components/development-discipline/skills/delivery-workflow/SKILL.md).
- Commit-message rationale: [rationale commit
  messages](../../components/development-discipline/skills/rationale-commit-messages/SKILL.md).
- Authoring or reviewing a skill: [writing
  skills](../../components/development-discipline/skills/writing-skills/SKILL.md).

Load `delivery-workflow` before the first implementation increment so the
configured delivery mode and green-increment preservation cadence are known
before any commit or push. Recheck it after final review for final delivery.

## Structured lifecycle

When a generated privileged profile is enabled, use only its structured
services. Never invoke Bash, apply-patch, raw Git, or forge argv as a fallback.

1. `workflow.start`: `production` for changed first-party shipped behavior;
   `exempt` only for a documented RED exemption.
2. Production: test author edits only its assigned test scope through
   `workspace-editor`; `project-runner` records the named failing command and
   `workflow.record_red` records RED, then the coordinator calls
   `workflow.authorize_implementation`.
3. The implementer changes only its assigned source scope. A named runner
   records GREEN through `workflow.record_green`; the coordinator calls
   `workflow.begin_verification`, then the verifier executes its declared
   verification command and passes that successful durable receipt to
   `workflow.record_verification` before review.
4. Complete `final_review`, then call `workflow.record_clean_review` with its
   compact `state_ref` as `review_state_ref` (or its exact legacy full state as
   `review_state`). A zero-assignment plan can route directly to this step.
   Finally call `workflow.authorize_delivery`, obtain repository receipts, and
   record delivery completion through `workflow.complete_delivery`.

An exemption skips only RED; verification, review, and delivery gates remain.
RED evidence requires executing one focused public or black-box behavior test
and confirming that it fails because the intended behavior is absent; a compile,
fixture, or setup failure is not valid RED. After authorization, implement only
enough to make that test pass. Refactor only after GREEN while the test remains
green, then repeat one observable behavior at a time.

The workflow transitions and assignments are mechanical, not user approval for
external actions. Every mutation must carry a current assignment with role,
state epoch, scope/command IDs, expiry, and configuration digest. A later
mutation invalidates verification/review evidence and returns the change to RED.

The `workflow.authorize_implementation` and
`workflow.authorize_delivery` calls are mechanical transitions, never user
approval for a destructive action, commit, push, pull request, merge, or
release. `production` means shipped behavior, not deployment.

To supersede a lifecycle, call `workflow.abandon`; never remove its state.

Before a new task, inspect forge CI. Require a successful completed run for the
candidate revision; queued/running is not terminal evidence, and any completed
failed required job activates a repository-wide hold. Use Tiber's single
CI-recovery owner and record SHA, run, checks, and terminal status. Development
Discipline reads that Tiber hold when gating delivery; it does not own one.
