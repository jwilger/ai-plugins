---
name: ci-failure-follow-up
description: Use when a pushed CI run fails or before later work or pushes; reproduce the exact recovery record without omissions, permit only a causal repair or unchanged-SHA rerun, keep separate defects outside the active ticket, and require terminal success.
---

# CI Failure Follow-up

A failed pushed run creates a hold. Stop unrelated implementation, review
remediation, ticket work, and all pushes except the one recovery action
selected below. Keep the hold until the failure is diagnosed and its
replacement run reaches terminal success.

There are exactly two recovery actions:

1. If the diagnosed failure requires a repository change, push only its tested
   causal repair. Its commit body must explain the diagnosed cause or risk. If
   the failure is unrelated to the active ticket, pause that ticket and make
   the separate repair the explicit recovery scope.
2. If evidence proves the failure unrelated or transient, rerun the exact
   unchanged SHA with no intervening push and keep any separate defect outside
   the active ticket. If that rerun fails, the hold remains and the failed
   rerun becomes the new failure record. Diagnose it; if it needs a repository
   change, transition to action 1 in a separate recovery scope rather than
   folding it into the paused ticket. Never fold it into the active ticket.

There is no diagnostic-commit path. Never push instrumentation, investigation,
no-op, speculative, or unrelated changes while the hold exists.

In advice, status, or handoff output, reproduce and complete this exact record;
do not collapse or omit a field:

```text
Failure record: <failed commit SHA>; <run ID or URL>; <exact failed job>;
  <failed step>; <relevant log evidence>
Diagnosis: <causal explanation>; classification=<caused|unrelated|transient>;
  <supporting evidence>
Next action: <tested causal repair whose next pushed commit body explains the
  diagnosis | rerun the unchanged revision without a no-op or intervening
  commit>
Release proof: <replacement run ID>; terminal status=<success>;
  queued|pending|running=<still blocked>
```

Persist this record in the active ticket's shared notes or the repository's
shared handoff state before ending a session. At session entry and before later
work or pushes, inspect the pushed CI runs for the active ticket since its first
pushed commit. Any failed run without a recorded terminal-success replacement
recreates the hold, even when a newer run is green or running.

An unrelated or transient classification never releases the hold by itself:
queued, pending, or running replacement CI is still blocked, and only terminal
success releases it.

## Recover the Run

1. Bind the failure to the exact pushed commit and CI run.
2. Inspect the exact failed job, the failed step, and its relevant logs. Do not
   infer the cause from the workflow or job title.
3. Record a causal diagnosis and the evidence that supports it.
4. If the diagnosed failure requires a repository change, write the focused
   test or check, repair only that cause, and make the next pushed commit the
   repair. Its rationale-bearing body must name the diagnosed failure or risk.
   For a defect unrelated to the active ticket, pause that ticket and track the
   causal repair as the explicit recovery scope.
5. If evidence proves the failure unrelated or transient, record that
   classification and keep the separate defect outside the active ticket.
   Rerun the unchanged source revision with no intervening push; never
   manufacture a no-op commit. If it succeeds, it releases the hold. If it
   fails, replace the failure record with that rerun, diagnose it, and either
   rerun the unchanged revision again with evidence or transition to the
   tested causal-repair action. No unrelated commit is allowed.
6. Poll the repair commit or evidence-backed rerun to terminal success. Queued
   or running is not repaired. Release the hold only after success.

The ordinary green-increment rule permits new work once CI is running or green
only when no failed-run hold exists. This recovery rule takes precedence after
a failure.

For example, if a marketplace canary names every loaded plugin but a checker
still rejects a capability description because it expected a literal skill
name, classify that as a checker mismatch with the captured output. File or
handle the checker defect separately; do not call it a loading failure or mix
its fix into the active ticket.
