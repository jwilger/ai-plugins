# sidequest / ai-plugins — canonical command interface.
# Run inside the Nix devshell (`nix develop`), e.g. `just ci`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Default: the full local gate (mirrors CI).
default: ci

# Full local quality gate.
ci: fmt-check clippy test

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

# Tests (CI gate). `--no-tests=pass` keeps the walking skeleton green.
test:
    cargo nextest run --workspace --all-features --no-tests=pass

# Mutation testing — 100% kill required (release-gated in CI).
mutants:
    cargo mutants --workspace

# Dependency vulnerability audit.
audit:
    cargo audit

# Marketplace manifest + formatting validation.
validate-marketplace:
    jq empty .claude-plugin/marketplace.json
    find plugins -name plugin.json -exec jq empty {} \;
    prettier --check "**/*.{json,md}"
