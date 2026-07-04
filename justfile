# ai-plugins — canonical command interface.
# Run inside the Nix devshell (`nix develop`), e.g. `just ci`.

set shell := ["bash", "-eu", "-o", "pipefail", "-c"]

# Default: the full local gate (mirrors CI).
default: ci

# Full local quality gate.
ci: validate-marketplace bats

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
