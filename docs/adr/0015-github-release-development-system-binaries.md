# ADR-0015: Distribute Development System binaries through GitHub Releases

## Status

Accepted; supersedes ADR-0013

## Date

2026-09-12

## Context

ADR-0013 removed compiled executables from Git history, but made every normal
installation compile two Rust workspaces locally. That imposed a Rust toolchain
and dependency download on users who only needed to start the packaged MCPs.
The XDG installation and stable project-local absolute paths already separate
runtime installation from the plugin checkout.

## Decision

Publish one `linux-x86_64` GitHub Release bundle for each synchronized
Development System plugin version. The repository release builder uses the
pinned Rust toolchain and `x86_64-unknown-linux-musl`, rejects binaries with an
ELF interpreter, dynamic-library dependencies, or `/nix/store/` references,
and starts both executables in a minimal environment before packaging them with
the AGPL license and exact source tag and commit. It emits a matching SHA-256
sidecar.

The default installer downloads that exact versioned bundle over HTTPS,
verifies the sidecar and fixed archive layout before extraction, and retains
the existing lock, staging directory, atomic symlink publication, and recovery
behavior. Unsupported hosts compile only when the caller explicitly supplies
`--from-source`.

A privileged workflow runs only for a successful CI `workflow_run` on the
exact current `main` commit. The build job has read-only repository permission
plus narrowly scoped provenance permissions; a separate publication job owns
release writes. Published releases are immutable: retries may repair drafts,
but conflicting tags or incomplete published releases fail.

## Consequences

Normal Linux x86_64 setup no longer needs Cargo. GitHub Releases, HTTPS,
`curl`, `tar`, `sha256sum`, and `flock` become runtime installation
dependencies. Other hosts retain a reproducible source-build route. Generated
binaries and archives remain outside Git history, while the XDG paths and
project-local MCP configuration contract do not change.

## Alternatives considered

Continuing mandatory local builds preserves fewer remote dependencies but
retains substantial setup cost. Committing binaries or archives makes clones
and repository history permanently larger. Publishing multiple targets now
would claim support that has not been built and accepted on those hosts.

## Revisit when

Add another prebuilt target only after its static build and clean-host startup
are reproducible and release acceptance covers it. Reconsider the channel if
GitHub Releases can no longer provide immutable, attestable artifacts.
