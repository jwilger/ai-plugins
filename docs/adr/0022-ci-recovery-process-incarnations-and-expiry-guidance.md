# 0022: CI recovery process incarnations and expiry guidance

Status: Accepted

## Context

ADR 0002's epoch and lease fencing protected shared recovery, but MCP restarts
changed fallback participant identity without actionable expiry guidance. Claim
continued returning waiting after expiry and wait did not wake at the deadline.
Harness session overrides could also give concurrent MCP servers equal authority.

## Decision

Use one unique MCP incarnation as the authority participant and expose the logical
harness session separately as a task label. Claim returns the process participant
for explicit transfer to a joined successor while the owner is alive. A vanished
owner retains its original sixty-minute lease; successors receive an explicit
retry timestamp. Claim/status/wait expose observed lease validity and takeover
availability. A bounded wait wakes at expiry without a new event. Expiry is only
guidance: successors explicitly take over at the observed epoch and assert the
new authority. Competing claims and takeovers retain normal EventCore publication
and epoch checks. CLI session identity remains explicit for existing callers.
An expired incumbent may also explicitly take over at the current epoch; this
advances the epoch and fences its previous authority exactly as successor takeover
does. An active lease never permits takeover, including by the incumbent.

## Consequences

Diagnosis, selected action, triggers, and evidence survive handoff and takeover.
Only exact successful replacement-run proof releases the hold. Missing local
PIDs cannot establish cross-host death. We reject automatic session resumption,
static shared identities, and shorter leases because they weaken concurrent
owner exclusion. Before expiry, disappearance can still require waiting; only a
live owner can authorize early transfer. Cross-host clock synchronization remains
an assumption from ADR 0002. Responses add lease fields and a takeover-available
role/wake reason; clients must accept them. Installed MCP processes and skills
must be refreshed to activate the change; existing incidents need no migration.
