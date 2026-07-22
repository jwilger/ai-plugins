# ai-plugins — canonical command interface.
# Run inside the Nix devshell (`nix develop`), e.g. `just ci`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Default: the full local gate (mirrors CI).
default: ci

# Full local quality gate.
ci: validate-marketplace github-actions tiber-rust development-discipline-rust development-discipline-release-from-source development-discipline-release-complete tiber-dashboard-smoke tiber-mutants tiber-release-complete bats

# Validate GitHub Actions syntax and semantics used by repository tests.
github-actions:
    actionlint

# Rust gates for the tiber plugin workspace.
tiber-rust:
    cargo fmt --manifest-path plugins/tiber/rust/Cargo.toml --all --check
    cargo clippy --manifest-path plugins/tiber/rust/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path plugins/tiber/rust/Cargo.toml

# Rust gates for the development-discipline MCP coordinator.
development-discipline-rust:
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-.dependencies/cargo-target/development-discipline}" cargo fmt --manifest-path plugins/development-discipline/rust/Cargo.toml --all --check
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-.dependencies/cargo-target/development-discipline}" cargo clippy --manifest-path plugins/development-discipline/rust/Cargo.toml --all-targets -- -D warnings
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-.dependencies/cargo-target/development-discipline}" cargo test --manifest-path plugins/development-discipline/rust/Cargo.toml

development-discipline-release-complete:
    cd plugins/development-discipline && sha256sum --check release-binaries.sha256
    bash scripts/check-development-discipline-release-complete.sh
    version="$(jq -r '.version' plugins/development-discipline/.codex-plugin/plugin.json)" && rg "development-discipline/${version}/bin/development-discipline-mcp" plugins/development-discipline/.mcp.json

development-discipline-release-from-source:
    bash scripts/check-development-discipline-release-from-source.sh

# Build every bundled development-discipline MCP release target.
development-discipline-release-all:
    scripts/build-development-discipline-release-all.sh

# Browser smoke coverage for the read-only tiber dashboard.
tiber-dashboard-smoke:
    scripts/evals/ensure-node-deps.sh
    node scripts/tiber/dashboard-smoke.mjs

# Build the tiber release binary for the current host target.
tiber-release-host:
    scripts/build-tiber-host-release.sh

# Build every bundled tiber v1 release target.
tiber-release-all:
    scripts/build-tiber-release-all.sh

# Mutation gate for the pure tiber core.
tiber-mutants:
    CARGO_MUTANTS_OUTPUT="${TMPDIR:-/tmp}/tiber-mutants" CARGO_TARGET_DIR="${TMPDIR:-/tmp}/tiber-mutants-target" cargo mutants --manifest-path plugins/tiber/rust/Cargo.toml --package tiber-core --test-workspace true

# Ensure the tiber release plan names every bundled v1 binary target.
tiber-release-manifest:
    bash scripts/check-tiber-release-manifest.sh

# Require every listed tiber release binary to be present and executable.
tiber-release-complete:
    bash scripts/check-tiber-release-complete.sh

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

# Local-only EMC devshell coverage. This intentionally does not run in CI.
emc-check:
    bats tests/emc-devshell.bats

# Install Lefthook-managed hooks for worktree bootstrap and main-checkout enforcement.
worktree-hooks:
    scripts/install-worktree-hooks.sh

# Fail unless the current checkout is a linked worktree suitable for agent edits.
agent-checkout-guard:
    scripts/agent-checkout-guard.sh

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
    bash scripts/check-advisor-agent-config.sh
    bash scripts/check-model-routing-config.sh
    prettier --check "**/*.{json,md}"
