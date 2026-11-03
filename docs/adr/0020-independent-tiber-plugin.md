# ADR-0020: Independently installable Tiber plugin

## Status

Accepted. Supersedes ADR-0018's experimental pi delivery decision.

## Date

2026-11-02

## Context

Tiber provides a Git-backed task board, CLI, MCP server, dashboard, and task
operations that are useful without Development System's workflow policies.
Bundling both executables and their installation state prevents independent
adoption. Separately installed plugin caches need not be sibling directories,
so moving a Rust path dependency alone would break installed source builds.

## Decision

- Create `plugins/tiber` as an independently installable marketplace plugin.
  It owns task operations, Rust sources, dashboard, MCP registration, launcher,
  signing-socket configuration, and executable installation.
- Development System owns development policy and retains first-class Tiber
  integration. Keep `development-system:tasks` as a thin compatibility route to
  Tiber's authoritative skills. Its `features.tiber` setting controls integration,
  without disabling an independently installed Tiber plugin. Standalone Tiber
  uses `.tiber.toml` and does not require `.development-system.toml`.
- Preserve CLI commands, MCP tool names, event formats, Git refs, signatures,
  review evidence, and CI-recovery authority. Development System continues
  reading Tiber's recovery hold rather than maintaining a second authority.
- Keep shared Rust infrastructure in this repository. Development Discipline
  may depend on Tiber's Git crate at compile time. Installed source fallback
  uses a checksum-verified exact-release source bundle containing transitive
  crate sources when a matching complete local source tree is unavailable;
  never infer dependency compatibility from another installed plugin cache.
- Install both plugins by default through the repository installer, while
  allowing direct Tiber-only marketplace installation. Give each plugin its
  own release assets, version, data namespace, lock, health checks, and atomic
  repair. Repairing one executable must not require reinstalling the other.
- Migrate signing-socket configuration to Tiber's verified plugin data. Keep
  existing board history intact. Existing CLI forwarders retain their
  no-overwrite contract and receive an explicit repair path.
- Remove experimental pi delivery, including its adapter, plugin npm package,
  runtime options, tests, and gates. Retain ADR-0018 as historical rationale and
  the root npm project used for Codex-backed evaluations.
- Release the extraction as Tiber 1.0.0 and Development System 10.0.0. Changes
  to shared crate behavior require versioning all affected executables.

## Consequences

Tiber can be adopted independently while combined development workflows retain
their existing guarantees. Per-plugin distribution needs separate repair and
release validation, plus complete source bundles for installed source builds.
No portable plugin dependency resolver is assumed: missing companions must
produce actionable diagnostics for operations that require them.

A separate repository, arbitrary task-tracker integration, changes to board
storage, and renewed support for other harnesses are outside this decision.

## Validation

Exercise standalone, combined, disabled-integration, and missing-companion
installations; independent upgrades and concurrent repairs; isolated-cache
source builds; existing boards and forwarders; signing, recovery holds, and
review receipts. Verify removal of active pi paths and use focused behavior
evaluations for skill routing and companion discovery.
