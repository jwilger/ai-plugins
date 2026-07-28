# ai-plugins — canonical command interface.
# Run inside the Nix devshell (`nix develop`), e.g. `just ci`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Default: the full local gate (mirrors CI).
default: ci

# Full local quality gate.
ci: validate-marketplace pi-extension github-actions tiber-rust development-discipline-rust development-discipline-release-from-source development-discipline-release-complete tiber-dashboard-smoke tiber-mutants tiber-release-complete bats

# Validate GitHub Actions syntax and semantics used by repository tests.
github-actions:
    actionlint

# Deterministic Pi package and TypeScript extension gate.
pi-extension:
    npx tsc -p plugins/development-system/tsconfig.json
    node --experimental-strip-types --test scripts/tests/pi-extension.test.mjs scripts/tests/pi-goal-mode.test.mjs scripts/tests/pi-guards.test.mjs scripts/tests/pi-worktrees.test.mjs scripts/tests/pi-references.test.mjs scripts/tests/pi-mcp-bridge.test.mjs scripts/tests/pi-review-child.test.mjs scripts/tests/pi-ci-hold.test.mjs scripts/tests/pi-eval-provider.test.mjs
    node scripts/pi-package-canary.mjs

# Clean-checkout Pi release canary (runs the documented bootstrap exactly).
pi-clean-canary:
    node scripts/pi-package-canary.mjs --clean-checkout

# Rust gates for the tiber plugin workspace.
tiber-rust:
    cargo fmt --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --all --check
    cargo clippy --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --all-targets -- -D warnings
    cargo test --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml

# Rust gates for the development-discipline MCP coordinator.
development-discipline-rust:
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-.dependencies/cargo-target/development-discipline}" cargo fmt --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --all --check
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-.dependencies/cargo-target/development-discipline}" cargo clippy --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --all-targets -- -D warnings
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-.dependencies/cargo-target/development-discipline}" cargo test --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml

development-discipline-release-complete:
    cd plugins/development-system/components/development-discipline && sha256sum --check release-binaries.sha256
    bash scripts/check-development-discipline-release-complete.sh
    version="$(jq -r '.version' plugins/development-system/.codex-plugin/plugin.json)" && rg "development-system/${version}/bin/development-discipline-mcp" plugins/development-system/components/development-discipline/.mcp.json

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
    CARGO_MUTANTS_OUTPUT="${TMPDIR:-/tmp}/tiber-mutants" CARGO_TARGET_DIR="${TMPDIR:-/tmp}/tiber-mutants-target" cargo mutants --manifest-path plugins/development-system/components/tiber/rust/Cargo.toml --package tiber-core --test-workspace true

# Ensure the tiber release plan names every bundled v1 binary target.
tiber-release-manifest:
    bash scripts/check-tiber-release-manifest.sh

# Require every listed tiber release binary to be present and executable.
tiber-release-complete:
    bash scripts/check-tiber-release-complete.sh

# Run executable provider-backed Pi guard scenarios in disposable repositories.
pi-guard-evals:
    node scripts/evals/run-pi-guard-scenarios.mjs

# Run only provider-backed evals mapped to behavior affected since origin/main,
# then upload/share fresh Promptfoo artifacts when the scope produced them.
evals:
    #!/usr/bin/env bash
    set +e
    marker="$(mktemp)"
    trap 'rm -f "$marker"' EXIT
    touch "$marker"

    scripts/evals/run-changed.sh
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

# Explicit, expensive research run across every case, condition, and harness.
# This is never the default validation path.
evals-all:
    scripts/evals/run.sh

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

# Validate the exact public npmjs.org payload without publishing it.
npm-package:
    node scripts/validate-development-system-npm-package.mjs

# Pack, extract, install, and load the npm artifact through the pinned Pi canary.
npm-package-canary:
    node scripts/development-system-npm-canary.mjs

# Marketplace manifest + formatting validation.
validate-marketplace:
    jq empty .claude-plugin/marketplace.json
    jq empty .agents/plugins/marketplace.json
    find plugins -name plugin.json -exec jq empty {} \;
    bash scripts/validate-manifests.sh
    node scripts/sync-development-system-metadata.mjs --check
    node scripts/validate-pi-package.mjs
    node scripts/generate-pi-support-docs.mjs --check
    node scripts/validate-development-system-npm-package.mjs
    bash scripts/check-advisor-agent-config.sh
    bash scripts/check-model-routing-config.sh
    node scripts/generate-development-system-agents.mjs --check
    prettier --check "**/*.{json,md}"
