# ADR-0001: Overall architecture of the sidequest system

## Status

Accepted

## Date

2026-06-29

## Context

We are building **sidequest**: from a main Codex/Claude Code session, `/side-quest`
launches a backgrounded agent in its own git worktree (seeded with the native
`/goal`) that delivers a change per project config (local merge / push to an
origin integration branch / PR or MR with optional auto-merge + babysitting). It
must survive the main session closing, be monitored/steered via harness-native
surfaces, and clean up fully when done — across **both Claude Code and Codex**
with minimal duplication. The full design and the research behind it are in the
approved plan (`using-deep-research-let-s-lively-donut`).

Key research result: no harness has a native background mode that simultaneously
survives session-close, stays steerable, and runs on a local worktree — but
**both can consume an external MCP server**.

## Decision

- A **Rust control plane**, published to crates.io + a Nix flake, in this monorepo
  under `crates/`, is the shared deterministic core. It is reached **primarily via
  an MCP server** that both harnesses consume; the harness is the user's UI.
- **Two crates** (see ADR for crate decomposition): a pure, dependency-free
  `sidequest-core` (so "no I/O in the core" is compiler-enforced) and a `sidequest`
  shell crate (interpreter + all I/O) exposing `sidequest-mcp` and `sidequest` bins.
- **Functional core / imperative shell** with a **Step/Trampoline effect pattern**:
  the pure core yields effect descriptions; the shell interprets them.
- A **forge-agnostic `Forge` port** (GitHub / Forgejo / GitLab, no preference)
  keeps forge differences in the shell.
- A shared flock'd **registry** is the "hive-mind" state: launched side-quests are
  provisioned with the MCP/CLI and self-report / `ask` / `complete` through it.
- The four plugins (`side-quest`, `worktrees`, `babysit-pr`, `engineering-standards`)
  are thin, cross-harness, and built on this core.
- The engineering regime (toolchain, lints, BDD, mutation, ADRs, CI/CD) mirrors
  John's established conventions; see `docs/rules/` and subsequent ADRs.

## Consequences

### Positive

- One deterministic implementation; harness differences confined to thin adapters.
- Compiler-enforced purity, clean mutation testing on the core, repeatable behavior.
- Backgrounding works on each harness without depending on unstable research-preview
  features as the core path.

### Negative

- The monorepo's CI must run both heavy Rust gating and marketplace validation.
- The MCP control plane is a moving part we own (no native equivalent exists).

## Alternatives Considered

### Prompt-driven portable skills only (no Rust core)

Rejected: weaker determinism/self-healing; John values repeatable, debuggable
processes, and the control-plane logic (registry, supervision, babysitting) needs
real code.

### Native-only backgrounding (subagents / Routines)

Rejected as the core path: subagents die with the session; Routines run in the
cloud on a fresh clone (no local worktree) and can't be steered mid-run. They
remain optional/fallback backends.

### Separate repo for the crate; gh-only forge

Rejected: John chose the monorepo; and forge support must be agnostic across
GitHub/Forgejo/GitLab with no preference.

## Revisit when

A harness ships a native survives-close + steerable + local-worktree background
mode, or the MCP control plane proves insufficient for monitoring/steering.

## Related

- The approved plan document (full design + research)
- Subsequent ADRs: crate decomposition, effect pattern, lint posture, testing
  strategy, toolchain, forge port, backgrounding strategy.
