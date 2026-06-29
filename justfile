# sidequest / ai-plugins — canonical command interface.
# Run inside the Nix devshell (`nix develop`), e.g. `just ci`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Default: the full local gate (mirrors CI).
default: ci

# Full local quality gate.
ci: build fmt-check clippy test bdd bats

# Build the library + binaries with no dev-dependencies, so feature
# unification from dev-deps can't mask a missing feature that would break a
# standalone `cargo build` / `cargo publish`. (Deliberately not `--all-targets`,
# which would pull dev-deps back in; clippy/test cover those.)
build:
    cargo build --workspace

# Apply formatting.
fmt:
    cargo fmt --all

# Check formatting (CI gate).
fmt-check:
    cargo fmt --all -- --check

# Lints as hard errors (CI gate).
clippy:
    cargo clippy --workspace --all-targets --all-features -- -D warnings

# Unit/integration tests (CI gate). `--no-tests=pass` keeps thin crates green.
# The cucumber target uses a custom harness (see `bdd`), so exclude it here.
test:
    cargo nextest run --workspace --all-features --no-tests=pass -E 'not binary(=cucumber)'

# BDD / Cucumber acceptance tests (CI gate). Custom harness, so not run by nextest.
bdd:
    cargo test --workspace --test cucumber

# Shell / plugin-script tests (CI gate).
bats:
    bats $(find plugins scripts -name '*.bats' | sort)

# Mutation testing — 100% kill required (release-gated in CI).
# Unset CARGO_TARGET_DIR (the devshell pins it to an absolute path): otherwise
# cargo-mutants builds *mutated* source into the shared dev target dir and
# corrupts its incremental fingerprints. Isolated here under mutants.out/.
mutants:
    env -u CARGO_TARGET_DIR cargo mutants --workspace

# Dependency vulnerability audit.
audit:
    cargo audit

# Marketplace manifest + formatting validation.
validate-marketplace:
    jq empty .claude-plugin/marketplace.json
    jq empty .agents/plugins/marketplace.json
    find plugins -name plugin.json -exec jq empty {} \;
    bash scripts/validate-manifests.sh
    prettier --check "**/*.{json,md}"
