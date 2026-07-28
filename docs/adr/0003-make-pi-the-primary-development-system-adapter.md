# ADR-0003: Make Pi the primary development-system adapter

## Status

Accepted

## Date

2026-07-27

## Context

The development system has canonical skills and mature Rust implementations for
Tiber and final-review coordination, but it previously exposed only Claude Code
and Codex product surfaces. Pi can load the same Agent Skills and provides
first-party TypeScript extension events, native tools, trusted local-TUI
interaction, and OpenAI subscription authentication. Rewriting the retained
Rust systems or maintaining a third copy of shared behavior would add risk.

## Decision

Package `plugins/development-system` directly as the Pi package and make its
existing eight `skills/*/SKILL.md` files the explicit public skill set. Keep one
machine-readable Pi support inventory and one package release record; validate
or generate every harness manifest and agent adapter from canonical sources.

Implement new orchestration in TypeScript with pure semantic cores and narrow
imperative adapters. Domain workflows that require evidence use a `step()` /
`resume()` boundary. Keep `.development-system.toml` authoritative and preserve
existing valid schema version 1 files.

Retain Tiber and development-discipline in Rust. A supervised, package-owned MCP
client dynamically discovers an approved subset of their schemas, rejects
collisions or ambiguous provider schemas, bounds requests and output, propagates
cancellation, and terminates its process group on session shutdown. No arbitrary
user MCP server is admitted.

Use Pi event interception for configured coordination-checkout, path, secret,
shell, delivery-mode, destructive-operation, and CI-recovery boundaries. Keep
bounded autonomous goal state in Pi session-branch custom entries, continue only
from the settled/idle boundary, and reserve explicit stale-ID-guarded terminal
tools rather than inferring completion from assistant prose. Only a
local Pi TUI confirmation is trusted for consequential approval. RPC, print, and
JSON modes fail closed for approval; guarded direct RPC bash is unsupported and
reported as such. This is protection against ordinary model and owner mistakes,
not a same-UID or operating-system sandbox.

Run authoritative final-review assignments in fresh no-session Pi children.
Map abstract roles to harness-specific provider/model routes, require structured
results and role/freshness/closure attestations, and leave every timeout,
cancellation, provider error, or malformed result unresolved.

Evaluate Pi through a repository custom Promptfoo provider over documented JSON
mode. Prepare one disposable, single-writer Pi home per package condition, copy
only the OpenAI subscription auth entry with mode 0600, grant explicit one-run
trust to repository fixtures, prove package extension provenance, verify source
auth integrity, and remove copied stores after execution. Present Pi before
Claude Code and Codex in reports.

## Consequences

### Positive

- Pi gains deterministic enforcement and trusted interaction without forking
  skill prose or rewriting mature Rust behavior.
- Local-path installation and later npm distribution use the same package root.
- Provider-free contracts cover package, lifecycle, process, mode, and guard
  behavior; provider-backed scenarios verify actual effects.
- Claude Code and Codex remain supported from the same canonical sources.

### Negative

- The extension depends on documented Pi 0.82 APIs and must maintain an explicit
  compatibility range.
- Some secondary harness protections remain instructional or hook-enforced, not
  equivalent to Pi interception.
- Dynamic MCP admission and child-process supervision add lifecycle code.
- Direct RPC bash cannot participate in guarded execution with the current Pi
  event API, so guarded RPC usage excludes that command path.

## Alternatives Considered

### Rewrite Tiber and development-discipline in TypeScript

Rejected because protocol overhead is negligible and rewrites would discard
mature state, locking, Git, and review semantics.

### Rely only on skills

Rejected because stochastic instructions cannot reliably block wrong-checkout
writes, bind setup approval, or enforce delivery policy.

### Restore the prior event-sourced SDLC implementation

Rejected because its broad workflow history and operating cost were
proportionally unjustified. This design adds only guards tied to concrete
failure boundaries.

## Revisit when

- Pi changes a required lifecycle, package, trust, or tool event contract;
- authenticated RPC approval or direct-bash interception becomes available;
- measured usage justifies a new guard or a retained-component migration; or
- the product threat model expands beyond a trusted single-owner workstation.

## Related

- `docs/pi-extension-prd.md`
- `plugins/development-system/README.md`
- `.agents/plugins/pi-support.json`
