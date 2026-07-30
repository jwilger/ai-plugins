---
name: ci-failure-follow-up
description: Use when an unexpected pushed CI run fails; coordinate one Beads ci-recovery molecule and merge-slot owner, permit only a tested causal repair or unchanged-SHA rerun, and require terminal success.
---

# CI Failure Follow-up

An unexpected terminal pushed-CI failure creates a repository-wide hold. Stop
unrelated implementation, review remediation, task work, and pushes until the
replacement reaches terminal success.

Create or pour one P0 `ci-recovery` issue labeled
`development-system:ci-recovery`, atomically claim it, and acquire the Beads
project merge slot. The claimed issue and merge slot are the shared ownership
record. Other agents may inspect, reproduce, edit, or test only when the owner
assigns that bounded work; they do not independently push, rerun, select an
action, or release the hold. Sync ownership and evidence through the configured
Dolt remote with `bd dolt pull` and `bd dolt push`.

If Beads or its Dolt remote is unavailable, preserve the hold and restore shared
coordination before pushing or rerunning. Local notes do not grant ownership.
Never force-push task or Dolt state.

There are exactly two recovery actions:

1. Push one tested causal repair whose commit message explains the diagnosed
   failure or risk.
2. When evidence shows the failure is unrelated or transient, rerun the exact
   unchanged SHA without an intervening or no-op commit.

The recovery molecule records the failed SHA and run, exact job and step,
bounded sanitized log evidence, causal explanation, classification
(`caused`, `unrelated`, or `transient`), selected action, replacement SHA/run,
and terminal outcome. Never persist raw logs or secrets.

A failed replacement remains in the same recovery molecule and requires a new
diagnosis and selected action. Queued, pending, running, canceled, and failed do
not release the hold. Resolve the workflow's CI gate only for the exact
replacement run's terminal success; then close the recovery issue, release the
merge slot, and push Dolt state before unrelated work resumes.

An intentional failure produced while testing an active `ci-workflow-slice` is
related implementation evidence, not a separate recovery incident. Keep that
slice open, revise the workflow, push a new authorized test SHA, and resolve its
gate only when the expected CI behavior succeeds.
