# ai-plugins — canonical command interface.
# Run inside the Nix devshell (`nix develop`), e.g. `just ci`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Default: the full local gate (mirrors CI).
default: ci

# Full local quality gate.
ci: pre-commit bats-expensive development-discipline-release-from-source

# Every deterministic local check suitable for a commit hook.  The
# source/distribution parity probe is deliberately CI-only because it performs
# a release build and black-box integration run.
pre-commit: validate-marketplace bootstrap-development-system node-tests github-actions beads-formulas development-discipline-rust development-discipline-release-complete bats-pre-commit

# Validate GitHub Actions syntax and semantics used by repository tests.
github-actions:
    actionlint

# Verify the remaining harness-neutral bootstrap, including the managed Beads tool.
bootstrap-development-system:
    scripts/bootstrap-development-system.sh

# Node-level behavior checks for managed development tools.
node-tests:
    node --test scripts/tests/development-tool-policy.test.mjs

# Validate and cook every installed Beads workflow formula.
beads-formulas:
    scripts/check-beads-formulas.sh

# Rust gates for the development-discipline MCP coordinator.
development-discipline-rust:
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-.dependencies/cargo-target/development-discipline}" cargo fmt --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --all --check
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-.dependencies/cargo-target/development-discipline}" cargo clippy --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml --all-targets -- -D warnings
    CARGO_TARGET_DIR="${CARGO_TARGET_DIR:-.dependencies/cargo-target/development-discipline}" cargo test --manifest-path plugins/development-system/components/development-discipline/rust/Cargo.toml -- --test-threads=1

development-discipline-release-complete:
    cd plugins/development-system/components/development-discipline && sha256sum --check release-binaries.sha256
    bash scripts/check-development-discipline-release-complete.sh
    version="$(jq -r '.version' plugins/development-system/.codex-plugin/plugin.json)" && rg "development-system/${version}/bin/development-discipline-mcp" plugins/development-system/components/development-discipline/.mcp.json

development-discipline-release-from-source:
    bash scripts/check-development-discipline-release-from-source.sh

# Build every bundled development-discipline MCP release target.
development-discipline-release-all:
    scripts/build-development-discipline-release-all.sh

# Run only provider-backed evals mapped to behavior affected since origin/main,
# with a global cap of eight target calls, then upload/share fresh Promptfoo
# artifacts when the scope produced them.
evals:
    #!/usr/bin/env bash
    set +e
    marker="$(mktemp)"
    trap 'rm -f "$marker"' EXIT
    touch "$marker"

    PROMPTFOO_MAX_CONCURRENCY=8 scripts/evals/run-changed.sh
    status=$?
    if [ "$status" -eq 124 ] || [ "$status" -ge 128 ]; then
      exit "$status"
    fi

    fresh_artifacts=0
    fresh_artifact_paths=()
    for artifact in evals/out/results.json evals/out/report.html evals/out/results.junit.xml; do
      if [ -f "$artifact" ] && [ "$artifact" -nt "$marker" ]; then
        fresh_artifacts=1
        fresh_artifact_paths+=("$artifact")
      fi
    done

    share_status=0
    if [ "$fresh_artifacts" -eq 1 ]; then
      scan_args=(--private-root evals/out)
      metadata=evals/out/generated/agentic-systems-engineering.behavior.metadata.json
      if [ -f "$metadata" ]; then
        scan_args+=(--metadata "$metadata")
      fi
      scan_status=0
      scripts/evals/scan-behavior-artifacts.sh \
        "${scan_args[@]}" -- "${fresh_artifact_paths[@]}" || scan_status=$?
      if [ "$scan_status" -ne 0 ]; then
        rm -f -- "${fresh_artifact_paths[@]}"
        echo "Skipping promptfoo share because artifact secret scanning failed." >&2
        exit "$scan_status"
      fi
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
# This is never the default validation path, but uses the same one-process
# global cap of eight target calls as `just evals`.
evals-all:
    PROMPTFOO_MAX_CONCURRENCY=8 scripts/evals/run.sh

# Run the plugin-instruction improvement loop with a plugin-only diff guard.
improve-plugins:
    scripts/evals/improve-plugins.sh

# Run the eval-harness improvement loop with an eval-only diff guard.
improve-evals:
    scripts/evals/improve-evals.sh

# Shell / plugin-script tests (CI gate).
bats:
    bats $(find plugins scripts -name '*.bats' | sort)

# Deterministic checks that are fast enough to execute before every commit.
# Code-quality runtime-harness tests and cross-target release construction stay
# in the complete CI gate below; all other Bats coverage remains here.
bats-pre-commit:
    bats $(find plugins scripts -name '*.bats' ! -path 'scripts/tests/evals-code-quality-*.bats' ! -path 'scripts/tests/development-discipline-release-integration.bats' | sort)

# Integration coverage that is intentionally excluded from the commit hook but
# remains mandatory in `just ci`.
bats-expensive:
    bats $(find scripts/tests -name 'evals-code-quality-*.bats' | sort) scripts/tests/development-discipline-release-integration.bats

# Local-only EMC devshell coverage. This intentionally does not run in CI.
emc-check:
    bats tests/emc-devshell.bats

# Install the Lefthook-managed post-checkout worktree bootstrap hook.
worktree-hooks:
    scripts/install-worktree-hooks.sh

# Tear down generated runtime state before removing a linked worktree.
worktree-teardown path:
    scripts/worktree-teardown.sh "{{ path }}"
    git worktree remove "{{ path }}"

# Marketplace manifest + formatting validation.
validate-marketplace:
    jq empty .claude-plugin/marketplace.json
    jq empty .agents/plugins/marketplace.json
    find plugins -name plugin.json -exec jq empty {} \;
    bash scripts/validate-manifests.sh
    node scripts/sync-development-system-metadata.mjs --check
    bash scripts/check-no-pi-support.sh
    bash scripts/check-advisor-agent-config.sh
    bash scripts/check-model-routing-config.sh
    node scripts/generate-development-system-agents.mjs --check
    prettier --check $(git ls-files --cached --others --exclude-standard -- '*.json' '*.md')
