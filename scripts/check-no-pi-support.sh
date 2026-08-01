#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
failed=false

report() {
  printf 'pi_retirement.violation %s\n' "$*" >&2
  failed=true
}

for retired_path in \
  package.json \
  .agents/plugins/pi-support.json \
  plugins/development-system/package.json \
  plugins/development-system/tsconfig.json \
  plugins/development-system/bin/development-system-pi \
  plugins/development-system/extensions/development-system \
  scripts/bootstrap-pi-package.sh \
  scripts/development-system-npm-canary.mjs \
  scripts/generate-pi-support-docs.mjs \
  scripts/pi-package-canary.mjs \
  scripts/validate-development-system-npm-package.mjs \
  scripts/validate-pi-package.mjs \
  scripts/evals/pi-provider.mjs \
  scripts/evals/prepare-pi-home.mjs \
  scripts/evals/run-pi-guard-scenarios.mjs \
  scripts/evals/run-pi-worktree-tui-scenario.mjs; do
  if [[ -e "$root/$retired_path" ]]; then
    report "retired_path=$retired_path"
  fi
done

# Pi-era design notes and regression fixtures are retained as historical
# evidence. This scan covers live repository surfaces so stale package/runtime
# references cannot survive the removal.
for retired_identifier in \
  '@earendil-works/pi-coding-agent' \
  'PI_CODING_AGENT_DIR' \
  'development-system-pi' \
  'piCompatibility' \
  'extensions/development-system/' \
  'development_system_worktree_' \
  'bootstrap-pi-package.sh' \
  'pi-package-canary.mjs' \
  'validate-pi-package.mjs' \
  'generate-pi-support-docs.mjs' \
  'pi-provider.mjs' \
  'prepare-pi-home.mjs' \
  'run-pi-guard-scenarios.mjs' \
  'run-pi-worktree-tui-scenario.mjs'; do
  matches="$(rg --hidden --line-number --fixed-strings \
    --glob '!.git/**' \
    --glob '!.worktrees/**' \
    --glob '!.dependencies/**' \
    --glob '!node_modules/**' \
    --glob '!docs/adr/**' \
    --glob '!docs/archive/**' \
    --glob '!docs/research/**' \
    --glob '!docs/pi-extension-prd.md' \
    --glob '!scripts/check-no-pi-support.sh' \
    --glob '!scripts/tests/**' \
    "$retired_identifier" "$root" || true)"
  if [[ -n "$matches" ]]; then
    printf '%s\n' "$matches" >&2
    report "retired_identifier=$retired_identifier"
  fi
done

if [[ "$failed" == true ]]; then
  exit 1
fi

printf 'pi_retirement.active_surfaces_clean\n'
