# ADR-0005: Use Beads and Dolt for task and workflow state

## Status

Accepted

## Date

2026-07-30

## Context

The package-owned task tracker accumulated release binaries, an MCP server,
custom synchronization, locking, a dashboard, CI incident state, skills, tests,
and harness-specific launchers. Maintaining those surfaces duplicated a mature
external system and repeatedly created integration defects.

Beads provides hash-safe issue IDs, atomic claims, dependencies, molecules,
formulas, gates, merge slots, worktree discovery, stable JSON output, lifecycle
priming, and a versioned Dolt backend. Its official Claude and Codex guidance
prefers direct CLI use over MCP when agents have shell access.

## Decision

Adopt Beads as the sole task and workflow authority for development-system.
Use its embedded Dolt backend by default and Dolt remotes for synchronization.
Treat the release manifest as the supported CLI/version authority. When an
enabled capability lacks a compatible managed binary, offer an explicit
user-global, no-sudo installation into `~/.local/bin`. The current Beads
minimum and target is `bd 1.1.2`. Do not require a standalone `dolt` CLI for
embedded mode; reserve it for an explicit server-mode or direct-database use
case.

Package declarative formulas for delivery, behavior-driven development,
documentation, CI-workflow testing, validation-only changes, focused BDD and
unit cycles, and CI recovery. Inject `bd prime` through Pi, Claude, and Codex
lifecycle adapters. Do not bundle Beads, expose it through the package MCP
manifest, or install its competing Git and harness hooks.

Retain one dry-run-first compatibility command that imports legacy task history
into Beads. Delete every other retired tracker implementation and installation
surface.

## Consequences

- Task and workflow state is durable, dependency-aware, and synchronized through
  Dolt rather than source branches or Markdown files.
- Formula dependencies replace custom development phase state machines.
- The Pi extension becomes smaller and retains only safety, worktree, delivery,
  goal, and final-review responsibilities.
- Downstream users explicitly approve installation or updates of a supported
  `bd` binary, and a config no-op still reconciles that enabled dependency.
- Tools-only reconciliation does not create a repository commit.
- Formula semantics, managed-tool installation, and Beads release compatibility
  become explicit validation surfaces.

## Alternatives

Continuing to repair the package-owned tracker was rejected because its
maintenance cost exceeded its differentiating value. A Beads MCP bridge was
rejected because it adds schema tokens, process supervision, and another
failure boundary without improving shell-capable harnesses. GitHub Issues alone
was rejected because it does not provide the local dependency-aware workflow
and compaction-recovery model required by coding agents.
