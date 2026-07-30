# ADR-0002: Coordinate pushed-CI recovery through Beads

## Status

Accepted (revised 2026-07-30)

## Context

Several agents and worktrees can observe the same unexpected terminal pushed-CI
failure. Independent diagnosis, repair pushes, or reruns can conflict, while a
newer green run can mask the unresolved failure. The repository now uses Beads
with Dolt for shared task and workflow state and should not maintain a second
incident protocol.

## Decision

Represent recovery as one P0 Beads issue poured from the `ci-recovery` formula
and labeled `development-system:ci-recovery`. Claim the issue atomically and
acquire the project merge slot before selecting a recovery action. All other
sessions pause unrelated work.

The molecule records the failed SHA and run, exact job and step, bounded
sanitized evidence, causal diagnosis, classification, selected action, and
replacement identity. The owner chooses exactly one tested causal repair or an
unchanged-SHA rerun. Helpers may inspect, reproduce, edit, or test only through
an explicit bounded assignment; they do not independently push, rerun, select
an action, or release the hold.

The workflow's CI gate closes only for exact terminal-success proof. The owner
then closes the recovery issue, releases the merge slot, and pushes Dolt state.
Queued, running, canceled, and failed outcomes retain the hold. If Beads or its
Dolt remote is unavailable, preserve the hold and restore shared coordination
before action.

An expected failure generated while testing an active `ci-workflow-slice` is
related test evidence, not a separate incident.

This replaces the former custom fenced-lease implementation. The simpler Beads
claim, merge slot, molecule, gate, and Dolt history are the authoritative shared
primitives.

## Consequences

- Recovery uses the same synchronized dependency graph and audit history as
  other work.
- The package deletes substantial custom incident, MCP, and Git-branch code.
- Beads claims do not reproduce the former epoch lease. Merge-slot ownership and
  explicit handoff are therefore mandatory operating rules.
- Remote unavailability can delay recovery, but private notes never become
  authority.

## Alternatives

Independent recovery remains unsafe. A separate custom incident service would
restore lease fencing but would duplicate Beads and recreate the maintenance
burden this decision removes. Force-pushing shared task state remains
prohibited.
