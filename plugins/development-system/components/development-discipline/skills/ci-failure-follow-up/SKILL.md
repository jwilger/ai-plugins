---
name: ci-failure-follow-up
description: Use when an unexpected pushed CI run fails; coordinate one Beads ci-recovery molecule and merge-slot owner, permit only a tested causal repair or unchanged-SHA rerun, and require terminal success.
---

# CI Failure Follow-up

An unexpected terminal pushed-CI failure creates a repository-wide hold. In
direct-to-trunk mode, the incremental watermark is the most recent completed run
for the configured trunk branch that reached a pass/fail outcome; outside a
hold, it must have passed. The hold starts the instant any completed run for
that branch fails unexpectedly. Queued, pending, or running runs neither replace
the watermark nor create a hold. A canceled run is non-evidence: it neither
passes nor fails, does not replace the watermark, and does not create or release
a hold. Stop unrelated implementation, review remediation, task work, and
pushes; do only causal recovery until the terminal success of either the exact
tested causal-repair revision or the authorized rerun of the exact unchanged
failed SHA.

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

Before choosing an action, inspect the exact failed job, failed step, and
relevant logs; do not infer the cause from a workflow or job title. Record only
a bounded, sanitized summary or authoritative-log reference, never raw logs or
secrets.

There are exactly two recovery actions:

1. Push one tested causal repair whose commit message body explains the
   diagnosed failure or risk. If the failure is unrelated to the active ticket,
   pause that ticket and make the repair a separate recovery scope.
2. When concrete evidence shows the failure is unrelated or transient, rerun
   the exact unchanged SHA without an intervening or no-op commit. Keep a
   separate checker defect outside the active ticket; a classification or a
   rerun request is not recovery proof.

There is no diagnostic-commit path. Never push instrumentation, investigation,
no-op, speculative, or unrelated changes while the hold exists.

## Durable recovery record

In advice, status, or handoff output, reproduce and complete this record in the
claimed Beads `ci-recovery` molecule; do not collapse or omit a field:

```text
Recovery: <Beads ci-recovery ID>; owner=<claimed|waiting>; merge slot=<held|unavailable>
Failure record: <failed commit SHA>; <run ID or URL>; <exact failed job>;
  <failed step>; <bounded sanitized log summary or authoritative-log reference>
Diagnosis: <causal explanation>; classification=<caused|unrelated|transient>;
  <supporting evidence>
Next action: <tested causal repair whose next pushed commit body explains the
  diagnosis | rerun the unchanged revision without a no-op or intervening
  commit>
Release proof: <exact tested causal-repair revision and run ID | authorized
  rerun of exact unchanged failed SHA and run ID>; terminal status=<success>;
  queued|pending|running=<still blocked>
```

Persist the record through Beads comments and the configured Dolt remote before
ending a session or handing it off. At session entry, restart, and before later
work or pushes, pull the shared record and inspect pushed CI runs for the active
ticket since its first pushed commit. Any failed run without a recorded
terminal-success result from either authorized recovery branch reconstructs the
unresolved hold, even when a newer run is green or running. Local notes and a
newer or running build never mask an earlier hold.

If an unchanged-SHA rerun fails, it becomes the new failure record. Diagnose it
again; its diagnosed, tested causal repair is the only permitted next push. If
the rerun exposes a separate checker defect, make that repair a separate
recovery scope rather than folding it into the paused active ticket. Keep the
same claimed recovery molecule and its evidence trail; do not open a parallel
hold merely to bypass its gate.

Queued, pending, running, canceled, and failed outcomes do not release the hold.
Resolve the workflow's CI gate only upon terminal success of either the exact
tested causal-repair revision or the authorized rerun of the exact unchanged
failed SHA, not a different or later revision. Then close the recovery issue,
release the merge slot, and push Dolt state before unrelated work resumes.

An intentional failure produced while testing an active `ci-workflow-slice` is
related implementation evidence, not a separate recovery incident. Keep that
slice open, revise the workflow, push a new authorized test SHA, and resolve its
gate only when the expected CI behavior succeeds.
