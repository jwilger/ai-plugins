# sidequest / ai-plugins — canonical command interface.
# Run inside the Nix devshell (`nix develop`), e.g. `just ci`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Default: the full local gate (mirrors CI).
default: ci

# Full local quality gate.
ci: fmt-check clippy test bdd bats

# Build everything.
build:
    cargo build --workspace --all-targets

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
    bats plugins/worktrees/tests

# Mutation testing — 100% kill required (release-gated in CI).
mutants:
    cargo mutants --workspace

# Dependency vulnerability audit.
audit:
    cargo audit

# Marketplace manifest + formatting validation.
validate-marketplace:
    jq empty .claude-plugin/marketplace.json
    jq empty .agents/plugins/marketplace.json
    find plugins -name plugin.json -exec jq empty {} \;
    prettier --check "**/*.{json,md}"
