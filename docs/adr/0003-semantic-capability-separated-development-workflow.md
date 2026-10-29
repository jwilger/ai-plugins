# ADR 0003: Keep development workflow advisory and evidence based

## Status

Accepted

## Decision

Development System is opt-in through a valid root `.development-system.toml`
with schema version 3. Configuration names repository-relative scopes and
named direct-argv commands, with validated paths and protected state.

Development Discipline exposes bounded repository inspection, explicitly
confirmed repository-local setup, and independent final-review coordination.
Plugin instructions and hooks are advisory: they do not establish caller
identity, isolate host tools, deny ordinary harness capabilities, or authorize
repository effects. Agents use ordinary host tools under current user and
repository authorization.

Setup previews the complete proposed configuration and applies the confirmed
proposal without changing global preferences. The host workflow records RED,
GREEN, independent review, hook-backed commits, exact-source verification, and
delivery through its durable checkpoint protocol. Evidence records do not
supply mutation authority or replace actual verification.

Final-review decisions are EventCore facts in a separate local Git-backed
authority. SQLite is a rebuildable report/projection. Exact source identities,
independent reviewer lifecycle, current tests, and required adjudication bind
review credit. Historical review and recovery records remain readable.

Tiber remains the issue tracker and sole structured authority for task-board
and CI-recovery facts. Development Discipline consumes its delivery hold
without maintaining a parallel incident authority.

## Consequences

Read-only inspection remains useful outside Git and under absent or malformed
configuration. Invalid configuration reports typed remediation without
claiming to disable ordinary tools. Independent review, host authorization,
configured hooks, and terminal CI evidence remain distinct requirements.

Earlier generated-agent profiles, hook-denial rules, and short-lived boundary
proofs did not establish a supported authority boundary and must not be
regenerated. The isolated code-quality benchmark was a separate, purpose-built
eval containment mechanism; it was retired on 2026-10-28 with its custom runner
and scorer (see ADR-0001).
