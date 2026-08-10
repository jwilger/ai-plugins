# ADR-0012: Enforce a workspace-wide strict Clippy allowlist

## Status

Accepted

## Date

2026-08-10

## Context

Copied UI code and new harness crates must share one reviewable quality floor;
grandfathered warnings hide defects and make future toolchain changes noisy.

## Decision

Every shipping crate inherits workspace lints. Enable `pedantic` and
`restriction` at warning priority -1, allow
`blanket_clippy_restriction_lints`, and fail
`cargo clippy --workspace --all-targets --all-features -- -D warnings`.
Forbid unsafe code. Library code contains no `unwrap`, `expect`, `panic`,
`todo`, or `unimplemented`.

Fix warnings where practical. Permit only narrowly scoped
`#[expect(clippy::lint_name, reason = "…")]`. Blanket
`#[allow(clippy::…)]`, unreasoned suppression, and grandfathered fork source
are prohibited. Workspace exceptions require an amendment to this ADR.
`clippy.toml` may set deterministic thresholds but cannot disable categories;
nursery lints are selected individually.

Add a repository check proving first-party crate inheritance and auditing
unapproved `allow` or `expect` attributes.

## Consequences

New warnings intentionally break the build until reviewed. Adapting upstream UI
may require substantial fixes, but the policy remains uniform and explicit.

## Alternatives considered

Default Clippy, blanket category suppression, and grandfathering copied code
were rejected because they create an unreviewed exception surface.

## Revisit when

The pinned Rust toolchain changes lint semantics; review each exception rather
than weakening categories wholesale.
