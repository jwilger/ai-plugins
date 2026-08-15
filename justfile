# ai-plugins — canonical command interface.
# Run inside the Nix devshell (`nix develop`), e.g. `just ci`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Default: the full local gate (mirrors CI).
default: ci

# Full local quality gate.
ci: validate-marketplace github-actions tiber-harness-rust tiber-rust development-discipline-rust tiber-dashboard-smoke tiber-mutants bats

# The developer gate runs before every commit. It deliberately excludes
# acceptance, release, browser, mutation, and shell suites; CI owns those after
# the push. It still runs formatting, linting, and every Rust unit-test target.
pre-commit: validate-marketplace github-actions
    bash tiber/scripts/check-lint-policy.sh
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/tiber-harness" cargo fmt --manifest-path tiber/Cargo.toml --all --check
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/tiber-harness" cargo clippy --manifest-path tiber/Cargo.toml --workspace --all-targets --all-features -- -D warnings
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/tiber-harness" cargo test --manifest-path tiber/Cargo.toml --workspace --all-features
    cargo fmt --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --all --check
    cargo clippy --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --workspace --lib
    cargo fmt --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --all --check
    cargo clippy --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --bin development-discipline-mcp -- --test-threads=1

# Validate GitHub Actions syntax and semantics used by repository tests.
github-actions:
    actionlint

# Rust gates for the standalone Tiber harness workspace.
tiber-harness-rust:
    bash tiber/scripts/check-lint-policy.sh
    node tiber/scripts/tests/probe-app-server-effective-authority.test.mjs
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/tiber-harness" cargo fmt --manifest-path tiber/Cargo.toml --all --check
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/tiber-harness" cargo clippy --manifest-path tiber/Cargo.toml --workspace --all-targets --all-features -- -D warnings
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/tiber-harness" cargo test --manifest-path tiber/Cargo.toml --workspace --all-features

# Verify the pinned app-server authority fixture against a locally generated schema.
tiber-app-server-fixture-check schema:
    bash tiber/scripts/check-app-server-authority-fixture.sh "{{schema}}"

# Rust gates for the tiber plugin workspace.
tiber-rust:
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/tiber-plugin" cargo fmt --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --all --check
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/tiber-plugin" cargo clippy --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --all-targets -- -D warnings
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-target}/tiber-plugin" cargo test --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml -- --test-threads=1

# Rust gates for the development-discipline MCP coordinator.
development-discipline-rust:
    cargo fmt --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --all --check
    cargo clippy --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml -- --test-threads=1

install-development-system-binaries:
    scripts/install-development-system-binaries.sh

# Browser smoke coverage for the read-only tiber dashboard.
tiber-dashboard-smoke:
    scripts/evals/ensure-node-deps.sh
    node scripts/tiber/dashboard-smoke.mjs

# Mutation gate for the pure tiber core.
tiber-mutants:
    CARGO_MUTANTS_OUTPUT="${TMPDIR:-/tmp}/tiber-mutants" CARGO_TARGET_DIR="${TMPDIR:-/tmp}/tiber-mutants-target" cargo mutants --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --package tiber-core

# Run provider-backed promptfoo evals locally, upload/share the latest result,
# and print the share URL. This sends eval data to the configured promptfoo
# sharing service.
evals:
    #!/usr/bin/env bash
    set +e
    marker="$(mktemp)"
    trap 'rm -f "$marker"' EXIT
    touch "$marker"

    scripts/evals/run.sh
    status=$?
    if [ "$status" -eq 124 ] || [ "$status" -ge 128 ]; then
      exit "$status"
    fi

    fresh_artifacts=0
    for artifact in evals/out/results.json evals/out/report.html evals/out/results.junit.xml; do
      if [ -f "$artifact" ] && [ "$artifact" -nt "$marker" ]; then
        fresh_artifacts=1
      fi
    done

    share_status=0
    if [ "$fresh_artifacts" -eq 1 ]; then
      scripts/evals/share.sh
      share_status=$?
    else
      echo "Skipping promptfoo share because no fresh eval artifacts were generated." >&2
    fi

    if [ "$status" -ne 0 ]; then
      exit "$status"
    fi
    exit "$share_status"

# Run the plugin-instruction improvement loop with a plugin-only diff guard.
improve-plugins:
    scripts/evals/improve-plugins.sh

# Run the eval-harness improvement loop with an eval-only diff guard.
improve-evals:
    scripts/evals/improve-evals.sh

# Shell / plugin-script tests (CI gate).
bats:
    bats $(find plugins scripts -name '*.bats' | sort)

# Install Lefthook-managed local checks and optional worktree bootstrap.
worktree-hooks:
    scripts/install-worktree-hooks.sh

# Tear down generated runtime state before removing a linked worktree.
worktree-teardown path:
    scripts/worktree-teardown.sh "{{path}}"
    git worktree remove "{{path}}"

# Marketplace manifest + formatting validation.
validate-marketplace:
    jq empty .claude-plugin/marketplace.json
    jq empty .agents/plugins/marketplace.json
    find plugins -name plugin.json -exec jq empty {} \;
    bash scripts/validate-manifests.sh
    bash scripts/check-model-routing-config.sh
    prettier --check "**/*.{json,md}"
