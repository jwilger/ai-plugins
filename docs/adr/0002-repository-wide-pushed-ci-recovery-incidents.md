# ADR-0002: Coordinate pushed-CI recovery through one fenced incident

## Status

Accepted

## Date

2026-07-25

## Context

Several agents and worktrees can observe the same terminal pushed-CI failure.
Without shared ownership, they can concurrently diagnose from different logs,
push incompatible repairs, rerun different revisions, or let a newer green run
mask an unresolved failure. A local handoff note is not sufficient across
worktrees or sessions.

The existing Tiber task board is shared Git state but task ownership is too
coarse for the short, safety-critical recovery window. Recovery needs one
owner, fencing against stale owners, a bounded recovery path, and a durable
release condition.

## Decision

Store CI-recovery facts in Tiber's EventCore Git store on the authoritative
`tiber` branch. Every agent that sees a terminal failed pushed run must claim or
join its one active incident before acting. The claim grants one owner a
60-minute, epoch-fenced lease; all other agents wait in bounded intervals unless
the owner assigns inspect, reproduce, edit, or test work. Helpers cannot push,
rerun, or choose a recovery action; proof closure is not a helper assignment.

The owner records the failed SHA/run, exact job and step, a bounded explicitly
sanitized log summary or authoritative-log reference, causal explanation, and
classification. It selects exactly one action: a causal repair or unchanged-SHA
rerun. It heartbeats roughly every 15 minutes, may transfer ownership
explicitly, and can be replaced only after lease expiry. Cross-host clocks are
expected to be reasonably synchronized; agents do not take over on a marginal
timestamp boundary. A failed
replacement remains in the same incident as the next failure record. Any joined
participant may record exact matching replacement-run proof with terminal
success; it is the only repository-wide hold release and does not grant owner
authority.

Tiber's EventCore store fetches and compares the remote branch, publishes only
normal fast-forward updates, and retries pure named commands after concurrent
updates. It never force-pushes. Development Discipline reads only Tiber's
unresolved hold when authorizing delivery; it has no CI-recovery event stream,
snapshot, or mutation API. If Tiber or the remote is unavailable, recovery
fails closed: agents retain the hold and restore shared coordination before
choosing an action, pushing, rerunning, or releasing.

## Consequences

### Positive

- One durable owner makes the repair-versus-rerun decision from shared evidence.
- Epochs prevent a timed-out or transferred owner from acting after replacement.
- Bounded helper work preserves useful parallelism without concurrent release
  or mutation authority.
- Terminal-success proof prevents nonterminal or unrelated runs from releasing
  the hold.

### Negative

- A transient remote outage blocks recovery mutations and may delay a repair.
- Agents must report enough incident data to make ownership and takeover
  auditable.
- CI recovery shares Tiber's durable EventCore publication path, so a
  Tiber-enabled project is required when this delivery hold is enabled.

## Alternatives Considered

### Independent per-worktree recovery

Rejected because concurrent reruns and repairs race, and local notes cannot
fence stale sessions or establish a single release condition.

### Use a separate workflow CI store

Rejected because two event authorities for one incident can diverge on owner,
epoch, replacement, or release. Tiber now models the recovery facts directly;
Development Discipline is only a consumer of the resulting delivery hold.

### Permit force-push conflict resolution

Rejected because it can discard a newer incident update and reauthorize stale
ownership. Normal fast-forward compare-and-retry preserves the authoritative
state.

## Revisit when

The repository adopts a trusted central incident service with equivalent
compare-and-swap, fencing, audit history, and an availability model that can
replace the Git coordination branch.

## Related

- ADR-0001
- `plugins/development-system/components/tiber/skills/tiber/SKILL.md`
- `plugins/development-system/components/development-discipline/skills/ci-failure-follow-up/SKILL.md`
