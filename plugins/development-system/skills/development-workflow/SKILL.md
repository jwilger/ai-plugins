---
name: development-workflow
description: Use when making repository changes, debugging, handling review feedback, verifying work, conducting final review, recovering CI, or deciding the next development lifecycle step.
---

# Development workflow

Read `.development-system.toml` before choosing a workflow. Treat it as the
project's single feature and delivery-policy source.

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
- Authoring or reviewing a skill: [writing
  skills](../../components/development-discipline/skills/writing-skills/SKILL.md).

## Mechanical lifecycle gate

When Development Discipline is installed, start its lifecycle before mutation:

1. `workflow.start`: `production` for changed first-party shipped behavior;
   `exempt` only for a documented RED exemption.
2. Production: add the focused test, `workflow.record_red`, then
   `workflow.authorize_implementation`.
3. `workflow.record_green`, then `workflow.authorize_review`.
4. Complete `final_review`, then call `workflow.record_clean_review` with its
   compact `state_ref` as `review_state_ref` (or its exact legacy full state as
   `review_state`). A zero-assignment plan can route directly to this step.
   Finally call `workflow.authorize_delivery`.

An exemption skips only RED; verification, review, and delivery gates remain.
RED evidence requires executing one focused public or black-box behavior test
and confirming that it fails because the intended behavior is absent; a compile,
fixture, or setup failure is not valid RED. After authorization, implement only
enough to make that test pass. Refactor only after GREEN while the test remains
green, then repeat one observable behavior at a time.

The `workflow.authorize_implementation`, `workflow.authorize_review`, and
`workflow.authorize_delivery` calls are mechanical transitions, never user
approval for a destructive action, commit, push, pull request, merge, or
release. `production` means shipped behavior, not deployment.

To supersede a lifecycle, call `workflow.abandon`; never remove its state.

Before a new task, inspect forge CI. Require a successful completed run for the
candidate revision; queued/running is not terminal evidence, and any completed
failed required job activates a repository-wide hold. Use one Development
Discipline recovery owner and record SHA, run, checks, and terminal status.
Tiber does not own this hold.
