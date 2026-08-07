---
name: ci-failure-follow-up
description: Use when a pushed CI run fails or before later work or pushes; coordinate multiple agents through the workflow-owned CI-recovery owner/lease, reproduce the exact recovery record, permit only a causal repair or unchanged-SHA rerun, and require terminal success.
---

# CI Failure Follow-up

A failed pushed run creates a hold. Stop unrelated implementation, review
remediation, ticket work, and all pushes except the one recovery action
selected below. Keep the hold until the failure is diagnosed and its
replacement run reaches terminal success.

Multi-agent ownership is coordinated only through the epoch-fenced incident on
the authoritative `origin/development-workflow` branch, never through local
notes or a task board. If that remote state is unavailable, fail closed
and take no recovery action.

## Claim the Repository-Wide Incident

Every agent that detects a terminal failed pushed-CI run claims it before
diagnosis, a rerun, a push, or a release:

Call `workflow.ci_recovery.claim` on the Development Discipline MCP with
`run_id`, `run_url`, `failed_sha`, `workflow`, and `git_ref`.

The result identifies the single incident, current epoch, and role. Only the
epoch-fenced owner diagnoses and selects the recovery action. Nonowners hold
and make bounded `workflow.ci_recovery.wait` calls; they may help only through an
owner assignment limited to inspect, reproduce, edit, or test. Helpers report
evidence and never push, rerun, or choose an action. Proof closure is not
helper work, but any joined participant may independently record exact matching
terminal-success proof; that objective closure does not grant diagnosis,
action, push, or rerun authority.

The owner heartbeats about every 15 minutes during the 60-minute lease, checks
ownership/epoch before owner actions, and transfers explicitly for a handoff.
A joined participant may take over only after expiry; transfer or takeover
creates a new epoch, invalidating the former owner's authority. Cross-host
clocks must be reasonably synchronized; do not take over on a marginal
timestamp boundary.

Development Discipline synchronizes this state on `origin/development-workflow`,
separate from any task board. It fetches and compares, publishes lease-fenced
updates, and retries after concurrent updates. Never force-push the branch. If
Development Discipline or `origin` is unavailable,
fail closed: preserve the global hold and do not diagnose, push, rerun, choose
an action, or release until shared coordination is restored. Local notes
preserve observations only; they do not grant ownership.

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
Incident: <workflow incident ID>; epoch=<current owner epoch>; role=<owner|waiting>;
  lease expiry=<timestamp>
Failure record: <failed commit SHA>; <run ID or URL>; <exact failed job>;
  <failed step>; <bounded sanitized log summary or authoritative-log reference>
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
work or pushes, inspect the pushed CI runs for the active change since its first
pushed commit. Any failed run without a recorded terminal-success replacement
recreates the hold, even when a newer run is green or running.

Record the state through the incident owner as well. A replacement run that
fails remains in this same incident: replace the exact job, step, log evidence,
cause, classification, and selected action, then recover it. Never open a
parallel incident for a failed recovery run.

An unrelated or transient classification never releases the hold by itself:
queued, pending, or running replacement CI is still blocked, and only terminal
success releases it.

## Recover the Run

1. Claim or join the repository-wide incident. The owner binds the failure to
   the exact pushed commit and CI run. Inspect the claim result immediately. If
   it fails, stop: do not inspect logs, diagnose, edit, test, rerun, push, or do
   unrelated work. Only retry the exact structured claim, read CI-recovery
   status to restore shared coordination.
2. The owner inspects the exact failed job, failed step, and relevant logs. Store
   only a bounded, explicitly sanitized summary or authoritative-log reference;
   never persist raw logs, credentials, tokens, or other secrets. Do not infer
   the cause from the workflow or job title.
3. Record the causal diagnosis, classification, and supporting evidence in the
   active incident and recovery record.
4. If the diagnosed failure requires a repository change, write the focused
   test or check, repair only that cause, and make the next pushed commit the
   repair. Only the owner pushes it; its rationale-bearing body must name the
   diagnosed failure or risk.
   For a defect unrelated to the active ticket, pause that ticket and track the
   causal repair as the explicit recovery scope.
5. If evidence proves the failure unrelated or transient, record that
   classification and keep the separate defect outside the active ticket.
   Rerun the unchanged source revision with no intervening push; never
   manufacture a no-op commit. If it succeeds, it releases the hold. If it
   fails, replace the failure record with that rerun, diagnose it, and either
   rerun the unchanged revision again with evidence or transition to the
   tested causal-repair action. No unrelated commit is allowed.
6. The owner records and polls the repair commit or evidence-backed rerun to
   terminal success. Queued or running is not repaired. Any joined participant
   may record the matching terminal-success proof through Development Discipline; this is the only
   hold release and does not grant recovery-action authority.

Before starting a new task, the ordinary green-increment rule requires the most
recently completed CI build to be successful. A newer queued or running build
does not replace that result, but any current build with a completed failed job
creates this hold immediately. This recovery rule takes precedence after a
failure.

For example, if a marketplace canary names every loaded plugin but a checker
still rejects a capability description because it expected a literal skill
name, classify that as a checker mismatch with the captured output. File or
handle the checker defect separately; do not call it a loading failure or mix
its fix into the active ticket.
