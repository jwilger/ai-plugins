# ADR 0003: Separate development workflow into semantic capabilities

## Status

Superseded in part by [ADR-0004](0004-tiber-standalone-authority.md).

## Decision

Development System is opt-in through a valid root `.development-system.toml`
with schema version 3. The configuration names repository-relative path scopes
and direct-argv project commands. It rejects shell entry points, Git/forge
programs, absolute and parent paths, protected policy/state paths, and symlink
escapes.

The plugin-wide Development Discipline MCP exposes repository inspection and
explicitly confirmed, repository-local setup. Plugin instructions and hooks are
advisory. They do not establish caller identity, isolate host tools, deny normal
Codex or Claude capabilities, or authorize repository effects. Setup does not
generate privileged agents, activation receipts, or repository launch wrappers
and does not modify global user configuration.

The workspace editor, project runner, repository-local, repository-remote, and
diagnostic services remain capability-separated reusable components. Each
revalidates a durable assignment containing role, state epoch, scope IDs,
command IDs, expiry, and configuration digest. Standalone Tiber owns their
registration, authorization, isolation, and execution. An ordinary host may
continue using its normal repository and delivery tools while Tiber is being
bootstrapped.

Lifecycle state has explicit RED, implementation authorization,
implementation, verification, review, delivery, and terminal states. A later
mutation invalidates verification/review evidence and returns to RED.
Checkpoints, index ownership, and semantic local/remote delivery receipts are
implemented only by their dedicated repository services.

Accepting RED or GREEN is one checked EventCore decision that emits the
accepted command evidence, lifecycle transition, and exact Git-index
checkpoint together. Wrong-test recovery is a separately confirmed two-phase
semantic operation. Preview hashes and index trees make concurrent overlap
fail closed, completed operations replay their original receipts, and unrelated
staged entries are never replaced by a full-index restore.

The Development Discipline workflow authority uses one project-scoped
EventCore stream for lifecycle transitions, assignments, epochs, command
receipts, and checkpoints. Every authoritative workflow fact must join an
EventCore-backed authority rather than a mutable sidecar record. Each
transition executes through an EventCore `ModelCommand` whose typed state is
folded from declared streams and covered by the strict model checker.

An EventCore command names domain intent. Its state contains only the facts
that command needs to decide, and a successful decision emits typed facts
caused by that intent. Generic write envelopes, whole-workflow snapshots, and
"state published" events are not substitutes for domain commands. JSON and
SQLite may exist only as rebuildable projections, one-time import inputs, or
bounded non-authoritative recovery journals.

Tiber is the sole structured authority for task-board and CI-recovery facts.
Development Discipline may consume Tiber's delivery hold but never keeps a
parallel CI incident.

## Consequences

- Read-only inspection remains useful outside Git and under absent or malformed
  configuration.
- Invalid configuration reports typed remediation but does not disable ordinary
  host tools.
- Semantic services remain testable and reusable without pretending that a
  host plugin supplies an authority boundary.
- Tiber owns authoritative identities, policies, effects, receipts, recovery,
  and delivery.
- Existing boundary-proof, activation, generated-agent, and hook-denial
  artifacts are obsolete experimental residue and must not be regenerated.

## Superseded boundary experiment

Earlier experiments used generated Codex agent profiles, a `PreToolUse` denial
hook, and short-lived proof and activation files. They demonstrated some
version-specific filtering behavior but not a stable, supported authority
boundary. Treating that behavior as enforcement made normal Codex work fail
closed when MCP startup or generated state drifted. ADR-0004 therefore rejects
plugin-owned enforcement and moves authority into standalone Tiber.

The independent code-quality benchmark sandbox is not part of this plugin
boundary experiment. It remains a purpose-built eval containment mechanism.
