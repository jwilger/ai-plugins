# Lints — strict, allowlist posture

Lint policy lives in the workspace `[lints]` tables in the root `Cargo.toml`
(inherited via `[lints] workspace = true`); there is no `clippy.toml`.

Posture, per <https://billylevin.dev/posts/clippy-config/>: turn the `pedantic`
and `restriction` (and `nursery`) groups **on as groups**, then decline individual
lints case by case — each `allow` is a **documented project-policy decision**, not
a convenience suppression.

- `unsafe_code = "forbid"`.
- Panic family is **denied**: `unwrap_used`, `expect_used`, `panic`,
  `indexing_slicing`, `unreachable`, `todo`, `unimplemented`. Production code uses
  `ok_or` / `?` / `ok_or_else`.
- Suppress only with `#[expect(clippy::lint, reason = "…")]` — `allow_attributes`
  and `allow_attributes_without_reason` are denied, so bare/`reason`-less `#[allow]`
  is rejected and stale suppressions surface.
- CI runs `cargo clippy --workspace --all-targets --all-features -- -D warnings`.
  Run `just clippy` before every commit.
