# ADR-0010: Route specialized agents by advertised capability

## Status

Accepted

## Date

2026-08-01

## Context

Specialized agents such as Advisor need the strongest eligible reasoning route,
but concrete model identifiers age quickly. A model fixed in a Codex agent file
can become unavailable or cease to be the strongest option while the static
configuration still appears valid. Guessing from model names or list order is
also unreliable because neither is a capability contract.

Codex can advertise eligible models and capability or upgrade metadata at
dispatch time. Claude Code provides moving family aliases and agent-level effort
configuration. The two harnesses therefore need different configuration shapes
for the same intent.

## Decision

Omit the model field from specialized Codex agent TOML when selection requires
the strongest eligible model. Make the invoking skill inspect authoritative
harness-advertised capability or upgrade metadata, rank only eligible models,
and pass the highest-capability selection explicitly when spawning the agent.
Fail visibly when authoritative ranking, explicit selection, or launch is
unavailable. Never infer ranking lexically or accept silent fallback.

Keep stable execution constraints in the agent definition. Advisor remains
read-only and uses `xhigh` reasoning regardless of the selected Codex model.

For Claude Code, use the moving `opus` family alias and the highest supported
agent effort. Treat the alias as a harness-maintained capability route, not a
versioned model pin.

Keep authority independent of routing. A stronger model, greater effort, or
specialized agent never authorizes writes, destructive operations, delivery, or
external side effects.

## Consequences

### Positive

- Specialized Codex routes follow current harness capabilities without a
  metadata release for each model change.
- Routing fails visibly when its capability guarantee cannot be established.
- Read-only and effort constraints remain reviewable in static agent files.
- Claude Code follows its strongest maintained family without a version pin.

### Negative

- Codex dispatch must inspect and interpret authoritative capability metadata.
- A harness that cannot rank or explicitly select eligible models cannot run the
  specialized route.
- Codex and Claude Code agent definitions are intentionally asymmetric.

## Alternatives Considered

### Pin a concrete model identifier

Rejected because availability and relative capability change faster than plugin
configuration, creating stale or unavailable routes.

### Choose the lexically greatest or newest-looking identifier

Rejected because naming and presentation order do not establish capability.

### Let the harness silently choose its default

Rejected because the caller could not prove that the specialized responsibility
ran on the required capability tier.

## Revisit when

- Codex exposes a stable semantic capability selector directly in agent TOML;
- Claude Code replaces moving family aliases with an equivalent capability
  contract; or
- supported harnesses standardize capability-ranked agent dispatch.

## Related

- [`../../plugins/development-system/skills/advisor/SKILL.md`](../../plugins/development-system/skills/advisor/SKILL.md)
- [`../../plugins/development-system/README.md`](../../plugins/development-system/README.md)
