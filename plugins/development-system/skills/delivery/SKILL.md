---
name: delivery
description: Use when committing, pushing, opening or babysitting a PR/MR, merging, or deciding whether verified work is complete under the configured delivery mode.
---

# Delivery

Read `[delivery]` in `.development-system.toml`.

## Detailed lifecycle routing

This shared skill carries the decision-critical delivery contract. If the
installed detailed component skill at
`components/development-discipline/skills/delivery-workflow/SKILL.md`
(`development-discipline:delivery-workflow`) is accessible, read it for the
full mechanics; inability to locate it must never delay or replace the
immediate delivery answer.

Route the active delivery phase to its named specialist: use
`development-discipline:verification-before-completion` and
`development-discipline:final-review` for readiness,
`development-discipline:rationale-commit-messages` for an authorized commit,
`babysit-pr` when an authorized PR/MR needs monitoring, and
`development-discipline:ci-failure-follow-up` for an unexpected pushed-CI
failure. The CI specialist owns the recovery hold; a delivery-mode change does
not bypass it.

When the configured value is unavailable, do not choose a mode. State that
`.development-system.toml` is authoritative and summarize all three modes so
the user knows what evidence is missing.

- `direct-to-trunk`: commit verified semantic increments and push the configured
  trunk branch. The most recent completed CI run for that branch that reached a
  pass/fail outcome must have passed, and no unresolved failure hold may exist.
  Newer queued, pending, or running runs do not replace that watermark. A
  canceled run is non-evidence: it neither passes nor fails, does not replace
  the watermark, and does not create or release a hold. An unexpected completed
  failure for the configured trunk branch immediately blocks unrelated work and
  permits only causal CI recovery until terminal success of either the exact
  tested causal-repair revision or the authorized rerun of the exact unchanged
  failed SHA. Ticket completion still requires terminal success for the exact
  final pushed SHA.
- `pull-request`: publish a branch, create or update one PR/MR, monitor required
  checks and review feedback, and finish only at the repository's configured
  terminal state.
- `local-only`: a current user restriction narrows any standing repository
  authorization. Local investigation, implementation, testing, verification,
  and proportionate local review may proceed; commit only when currently
  requested or locally required. Do not push or publish, create or update a
  PR/MR, merge, force-push, or otherwise mutate remote state. Do not claim or
  imply that remote CI ran, that a remote delivery state exists, or that local
  evidence proves either one.

Always state each protected action explicitly: never infer permission to
force-push, merge, or delete remote state.

Worktree cleanup is optional housekeeping, not a delivery condition. If a
linked worktree was created solely to isolate concurrent mutable tickets and no
session still needs it, finish it only after terminal delivery evidence and a
clean status. A dirty worktree or cleanup failure must never be forced, but it
does not invalidate otherwise complete delivery.
