# ai-plugins — canonical command interface.
# Run inside the Nix devshell (`nix develop`), e.g. `just ci`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Default: the full local gate (mirrors CI).
default: ci

# Full local quality gate.
ci: validate-marketplace taskbranch-rust taskbranch-dashboard-smoke taskbranch-mutants taskbranch-release-manifest bats

# Rust gates for the taskbranch plugin workspace.
taskbranch-rust:
    cargo fmt --manifest-path plugins/taskbranch/rust/Cargo.toml --all --check
    cargo clippy --manifest-path plugins/taskbranch/rust/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path plugins/taskbranch/rust/Cargo.toml

# Browser smoke coverage for the read-only taskbranch dashboard.
taskbranch-dashboard-smoke:
    scripts/evals/ensure-node-deps.sh
    node scripts/taskbranch/dashboard-smoke.mjs

# Build the taskbranch release binary for the current host target.
taskbranch-release-host:
    scripts/build-taskbranch-host-release.sh

# Build every bundled taskbranch v1 release target.
taskbranch-release-all:
    scripts/build-taskbranch-release-all.sh

# Mutation gate for the pure taskbranch core.
taskbranch-mutants:
    CARGO_MUTANTS_OUTPUT="${TMPDIR:-/tmp}/taskbranch-mutants" CARGO_TARGET_DIR="${TMPDIR:-/tmp}/taskbranch-mutants-target" cargo mutants --manifest-path plugins/taskbranch/rust/Cargo.toml --package taskbranch-core --test-workspace true

# Ensure the taskbranch release plan names every bundled v1 binary target.
taskbranch-release-manifest:
    bash scripts/check-taskbranch-release-manifest.sh

# Require every listed taskbranch release binary to be present and executable.
taskbranch-release-complete:
    bash scripts/check-taskbranch-release-complete.sh

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
    if [ "$status" -ge 128 ]; then
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

# Install shared git hooks for worktree bootstrap and main-checkout enforcement.
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
    prettier --check "**/*.{json,md}"
