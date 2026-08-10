# ADR 0003: Enforce development workflow through semantic capabilities

## Status

Accepted.

## Decision

Development System is opt-in through a valid root `.development-system.toml`
with schema version 3. The configuration names repository-relative path scopes
and direct-argv project commands. It rejects shell entry points, Git/forge
programs, absolute and parent paths, protected policy/state paths, and symlink
escapes.

The plugin-wide Development Discipline MCP is read/setup-only. It exposes
status, preview, and an explicitly confirmed setup application. It does not
infer effects from tool names, patch content, command strings, shell segments,
redirects, wrappers, or Git/forge arguments. Tiber is the sole structured
authority for task-board and CI-recovery facts; Development Discipline consumes
only Tiber's delivery hold and never keeps a parallel CI incident. `PreToolUse:
"*"` workflow hooks are removed.

Mutating services are separate capability endpoints: workspace editor, project
runner, repository-local, repository-remote, and diagnostics. Each must
revalidate a durable assignment containing role, state epoch, scope IDs,
command IDs, expiry, and configuration digest. The service registration is
allowed only when a harness proof establishes both per-agent MCP isolation and
an OS write boundary. Until then, generated Claude and Codex profiles remain
read-only and mutation is unavailable rather than falling back to Bash,
`apply_patch`, raw Git, or forge argv.

Lifecycle state has explicit RED, implementation authorization, implementation,
verification, review, delivery, and terminal states. A later mutation invalidates
verification/review evidence and returns to RED. Checkpoints, index ownership,
and semantic local/remote delivery receipts are implemented only by their
dedicated repository services.

Accepting RED or GREEN is one checked EventCore decision that emits the
accepted command evidence, lifecycle transition, and exact Git-index
checkpoint together. Wrong-test recovery is a separately confirmed two-phase
semantic operation: `AuthorizeCheckpointAbort` folds the current phase, last
checkpoint identity/tree, paths changed since that checkpoint, and used
operation IDs; the repository-local boundary archives those paths under the
Git common directory and restores only their worktree/index entries;
`CompleteCheckpointAbort` then records the bounded receipt and returns the
lifecycle to RED while invalidating later evidence. Preview hashes and index
trees make concurrent overlap fail closed. Replaying a completed operation
returns its original receipt. Unrelated staged entries are never replaced by a
full-index restore.

The live Development Discipline workflow authority uses one project-scoped
EventCore stream for lifecycle transitions, assignments, epochs, command
receipts, and checkpoints. Every remaining workflow fact—including evidence,
blockers, delivery receipts, and final-review facts—must join an
EventCore-backed authority rather than a mutable sidecar record.
Each transition must execute through an EventCore experimental `ModelCommand`
whose typed `ModelState` data is evolved from its declared stream. Its inputs,
commands, events, mappings, and state fields must be registered in the strict
model checker with no accepted assumptions. The complete checker must run for
every Development Discipline command/state lane and every independently
governed Tiber command/state lane; a passing model for one helper is not
evidence for the rest.

An EventCore command names domain intent. Its state contains only the facts
that specific command needs to decide; folding relevant events mutates that
state. A successful decision emits typed facts caused by that intent. Generic
write-envelope commands, event-log command state, whole-workflow snapshots,
and “state published” events do not meet this requirement. The only domain
write path is EventCore `execute()`; mutating a projection then diffing it into
events, manually validating a caller-built event, or appending an arbitrary
event list is not an equivalent implementation.

EventCore is not modeled or described in aggregate terms. Domain authority is
expressed by commands whose own purpose-specific state is folded from their
declared streams.

This is an active migration requirement, not a claim that legacy state is
already compliant. Until a lane has typed facts, typed command provenance, a
checked command/state model, and an EventCore-compatible Git-backed authority, it is
incomplete. JSON/SQLite may exist only as a rebuildable projection, one-time
legacy import input, or bounded non-authoritative crash-recovery journal. In
particular, a typed wrapper around a whole-state snapshot is an interoperability
step, not the final per-transition model.

## Consequences

- Read-only inspection remains useful outside Git, without configuration, and
  under malformed/stale configuration.
- Invalid configuration fails closed for mutation and reports typed remediation.
- Tiber owns CI-recovery CAS and bounded receipt behavior as a repository
  service concern; it is not an approval bridge for arbitrary shell execution.
- Ordinary unit and integration tests complement, but do not replace, the
  strict checked-model gate for each EventCore command/state lane.
- The initial implementation intentionally withholds privileged profiles until
  the disposable Claude/Codex boundary spike supplies evidence.

## Boundary-spike record

On 2026-08-08, a Claude Code 2.1.226 session loaded the public plugin through
`--plugin-dir`; the repository-local 3.3.0 marketplace plugin was subsequently
installed and validated successfully. After the MCP command was changed to use
`${CLAUDE_PLUGIN_ROOT}` and the SessionStart hook was changed to the current
`{ "hooks": { ... } }` wrapper schema, both Development Discipline and Tiber
connected. A repeat non-interactive probe on the same Claude version advertised
only `workspace-reader.{status,list,read,search}` and
`setup.{preview,apply}` from Development Discipline; it advertised no workspace
editor, runner, repository, or diagnostics mutation service. This establishes
the direct read/setup launch surface. Claude nevertheless still exposes its
built-in Bash, Edit, and Write tools, plus independently governed Tiber
mutations; the SessionStart command also could not create Claude's per-session
environment directory under the evaluator's read-only home mount. This is not
a per-agent containment, caller-identity, or OS write-boundary proof.
An auto-discovered project subagent also received its built-in read/search
surface but did not receive an inline privileged MCP registration.
Development Discipline privileged profiles therefore remain unavailable for
Claude.

The installed Codex CLI 0.147.0 loads the repository-local 3.3.0 plugin. Direct
ephemeral probes established that the plugin-wide surface contains only reader
and setup tools, and that a manually supplied specialist configuration can keep
built-in writes inside a read-only OS sandbox while exposing only its named MCP
tools. The generated `.codex/agents/*.toml` files use Codex's current standalone
agent-role schema and are auto-discovered. However, the authenticated live
spawn of the generated test-author failed inside the collaboration thread
before producing a trustworthy child tool inventory; the fallback report did
not contain the configured specialist MCP services. A valid configuration file
and a manually loaded role do not prove the spawned-role boundary. Per-agent
MCP filtering and stable caller identity therefore remain unproved, and Codex
privileged profiles remain unavailable. A successful authenticated spawned-role
probe remains necessary
for installed-marketplace testing; Codex remains read-only and Development
Discipline mutation remains unavailable.
