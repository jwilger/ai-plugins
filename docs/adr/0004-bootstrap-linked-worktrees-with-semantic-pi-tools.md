# ADR-0004: Bootstrap linked worktrees with semantic Pi tools

## Status

Superseded in part by ADR-0005

## Date

2026-07-28

## Context

ADR-0003 made Pi's process working directory the authoritative enforcement
checkout and made a configured primary checkout coordination-only. The initial
guard correctly blocked ordinary primary-checkout mutation, but it also blocked
the mutation required to leave that checkout. Its recovery text required a
linked worktree without providing a safe way to discover or create one.
Command-level `cd` and `git -C` could not repair the mismatch because Pi's tool
API has no per-call working-directory override and the current process remains
bound to its startup checkout.

The same broad path and shell boundaries blocked the authoritative project
policy, installed Pi references required by repository instructions, and
bounded read-only checkout discovery. Setup preview rebuilt defaults instead of
preserving unspecified existing values. Stale goal ownership and cancelled
review children returned codes without enough state to recover.

## Decision

Keep the process checkout authoritative and retain the coordination-only write
boundary. Add first-party semantic tools that list registered worktrees and
create one new linked worktree from the primary HEAD. The create tool accepts
parsed worktree-name and branch types rather than shell fragments, derives the
path from the configured repository-local worktree root, invokes Git with an
argument array, and returns a canonical path plus an exact command for starting
a new Pi process there. It never claims that the current session moved.

Validate lexical and canonical containment before mutation. Reject absolute or
traversing roots, option-like or malformed values, NUL and control characters,
invalid refs, and symlink escapes. Preserve all existing collision state. Queue
creation in-process per repository and rely on Git's own metadata locking for
cross-process safety; after an external race or partial failure, reconcile the
observed worktree state and perform no destructive cleanup. Retain only one
strictly parsed shell compatibility form for worktree creation.

Admit direct status inspection only for canonical paths returned by Git's
worktree inventory. A bounded `cd <registered-worktree> && git status` or
`git -C <registered-worktree> status` command remains read-only and never
changes later enforcement. Expose dedicated readers for the canonical
`.development-system.toml` and a fixed allowlist of installed Pi documentation;
do not weaken arbitrary metadata, secret, or outside-path access.

For existing setup, derive the proposal from the current policy and patch only
explicit delivery or feature arguments. Bind the resulting source to repository
preconditions and apply it atomically with file and index rollback on commit
failure.

Expose current non-secret goal ID, guard epoch, status, and consumed bounds when
a terminal call is stale. Emit review-child start progress and return structured
lifecycle and cancellation diagnostics without stderr, environment, or auth
content. Keep full streaming subagent observability outside this repair under
Tiber ticket `20260728-9rym`.

## Consequences

### Positive

- An agent can follow worktree policy from a primary checkout without arbitrary
  shell mutation or an impossible recovery instruction.
- Relaunch requirements and enforcement identity are explicit and testable.
- Configuration and documentation reads are narrow capabilities rather than
  broad path exemptions.
- Setup, stale-goal, and child-cancellation failures provide deterministic
  recovery evidence.

### Negative

- The extension owns additional Git parsing, collision, and lifecycle code.
- Existing Pi processes cannot migrate in place; users and agents must launch a
  new process in the returned checkout.
- Cross-process creators are reconciled rather than governed by a package-owned
  global lock, so Git remains the metadata-locking authority.
- Full child output streaming and richer multi-agent observability remain
  deferred.

## Alternatives Considered

### Allow arbitrary shell mutation in the primary checkout

Rejected because it removes the coordination boundary instead of repairing its
bootstrap path.

### Treat `cd` or `git -C` as a checkout migration

Rejected because those commands affect one subprocess only and cannot change
Pi's authoritative extension context.

### Automatically relaunch or replace the current Pi process

Rejected because Pi exposes no safe extension API for changing the process
working directory, and implicit process replacement would obscure trust and
session ownership.

### Use only the compatibility `git worktree add` shell form

Rejected as the primary interface because shell syntax expands the injection
and ambiguity surface and cannot return a stable structured recovery contract.

## Related

- `docs/adr/0003-make-pi-the-primary-development-system-adapter.md`
- `docs/adr/0005-switch-pi-sessions-between-registered-worktrees.md`
- `docs/pi-extension-prd.md`
- `plugins/development-system/README.md`
- Tiber ticket `20260728-9rym`
