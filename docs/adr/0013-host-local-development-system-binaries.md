# ADR-0013: Build Development System binaries on the installed host

## Status

Accepted

## Date

2026-08-14

## Context

Development System previously committed cross-platform Rust executables and
their release checksums to the marketplace. That made ordinary clones large and
left launchers with a Cargo runtime fallback, despite normal use not being a
development workflow.

## Decision

Ship source and thin launchers only. The explicit
`just install-development-system-binaries` command builds the current host with
the repository's pinned, locked Rust toolchains and atomically installs both
executables under the XDG data directory, keyed by Development System version
and host. It requires working Cargo; Nix is an optional repository development
environment. Launchers resolve that exact location and fail with the bootstrap
command when it is absent; they never compile at runtime.

Remove release bundles, cross-target builders, release checksums, and
source-fingerprint parity checks. This change performs no remote mutation. A
separate history-purge operation may remove the former binary paths from every
reachable ref only after a coordinated freeze, an exact ref inventory, durable
backup and recovery evidence, and explicit case-specific authorization. It must
preserve the authoritative `tiber` branch unless its separately authorized
handling is explicitly planned and verified.

## Consequences

Initial installation requires a local Rust build and any required dependency
downloads. Plugin upgrades keep prior versioned installations available, while
normal MCP and CLI startup has no Cargo dependency. Releases no longer provide
prebuilt cross-platform payloads.

## Alternatives considered

Keeping bundled binaries retains the clone-size and history cost. Keeping a
Cargo fallback makes normal startup unpredictable and turns a missing install
into an implicit build. Both were rejected.

## Revisit when

Revisit only if the marketplace needs an installation channel that cannot build
Rust locally and can provide an artifact distribution system without committing
large executable payloads to this repository.
