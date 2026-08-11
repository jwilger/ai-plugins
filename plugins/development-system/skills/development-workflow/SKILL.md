---
name: development-workflow
description: Use when making repository changes, debugging, handling review feedback, verifying work, conducting final review, recovering CI, or deciding the next development lifecycle step.
---

# Development workflow

Use `workspace-reader.status` before choosing a workflow. Outside Git, without
configuration, or with invalid configuration, inspection remains available but
the plugin cannot provide configured workflow guidance. This advisory state
does not deny ordinary host mutation capabilities.

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

When a `final_review.plan` handoff is truncated, recover it instead of
replanning. Call `final_review.pending_assignments` with the retained
`state_ref`; its compact summary is authoritative for each pending lens's
iteration, exact `subagent_key`, exact `model_role`, `close_after_result`,
shared-test-evidence ID, and result-schema version. Pass one exact
`subagent_key` back to that tool to retrieve only that assignment's full prompt
and schema. An incomplete `final_review.resume_latest` returns the same compact
summary. These retrievals are read-only and idempotent: they append no event,
change no revision, fingerprint, scope, or hash, and never reassign a lens or
role. On a role mismatch, rerun only the named assignment in fresh context with
the reported expected role, then resubmit it without restarting unrelated clean
lenses.

The pinned Git baseline plus in-repository paths, contents, modes, and untracked
inventory define reviewed source scope. Coordinator events, snapshots, state
files, and projections are bookkeeping outside that hash. After terminal source
review, a content-identical commit does not reopen review merely because HEAD,
staging partition, signature, or commit metadata changed. Real path, content,
mode, untracked-content, pinned-baseline, or requested-scope changes do reopen
it. The one-way delivery sequence remains: source-content review, commit, a
fresh full gate bound to that exact commit, then push and exact-revision CI.

Load `delivery-workflow` before the first implementation increment so the
configured delivery mode and green-increment preservation cadence are known
before any commit or push. Recheck it after final review for final delivery.

## Structured lifecycle

The bootstrap plugin exposes structured `final_review.*` coordination, but not
the native `workflow.*`, `workspace-editor.*`, or `project-runner.*` services.
Use ordinary host tools for authorized repository changes and verification, and
follow the RED-GREEN-refactor, exact-evidence, review, and delivery guidance in
the specialist contracts above. Do not claim that plugin instructions, hooks,
or final-review records enforce harness capabilities or authorize external
actions.

The retained native lifecycle is for standalone Tiber. There, assignment-bound
editor and runner receipts support the mechanical RED, GREEN, verification,
review, and delivery transitions: `workflow.record_red`,
`workflow.authorize_implementation`, `workflow.record_green`,
`workflow.begin_verification`, `workflow.record_verification`,
`workflow.record_clean_review`, `workflow.authorize_delivery`, and
`workflow.complete_delivery`. Do not call or emulate those native transitions
through the advisory bootstrap surface.

Before a new task, inspect forge CI. Require a successful completed run for the
candidate revision; queued/running is not terminal evidence, and any completed
failed required job activates a repository-wide hold. Use Tiber's single
CI-recovery owner and record SHA, run, checks, and terminal status. Development
Discipline reads that Tiber hold when gating delivery; it does not own one.
