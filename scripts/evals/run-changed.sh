#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../.." && pwd)"
base_ref="${EVAL_BASE_REF:-origin/main}"

usage() {
  cat <<'USAGE'
Usage: scripts/evals/run-changed.sh [--base REF] [--dry-run]

Runs only provider-backed evals whose measured behavior can be affected by the
changed files between REF and HEAD. The default REF is origin/main.

Selection rules:
  shared skill prose     matching behavior cases, all supported harnesses, k=1
  Pi package/runtime     targeted Pi installed-package canary
  Pi guard/runtime       executable Pi guard outcome scenarios
  docs/tests/other code  no provider-backed eval

Use scripts/evals/run.sh directly with explicit case/provider filters for a
manually selected scope. The intentionally exhaustive matrix is `just evals-all`.
USAGE
}

dry_run=0
while [ "$#" -gt 0 ]; do
  case "$1" in
    --base)
      base_ref="$2"
      shift 2
      ;;
    --dry-run)
      dry_run=1
      shift
      ;;
    --help)
      usage
      exit 0
      ;;
    *)
      echo "unknown option: $1" >&2
      usage >&2
      exit 2
      ;;
  esac
done

cd "$root"
merge_base="$(git merge-base "$base_ref" HEAD)" || {
  echo "cannot determine eval change base from $base_ref" >&2
  exit 2
}
mapfile -t changed < <(git diff --name-only "$merge_base" HEAD)

if [ "${#changed[@]}" -eq 0 ]; then
  echo "eval scope: no committed changes since $base_ref; no provider evals selected"
  exit 0
fi

skill_names=()
pi_package=0
pi_guards=0
pi_goal=0
pi_worktree_tui=0
for file in "${changed[@]}"; do
  case "$file" in
    plugins/development-system/skills/*/SKILL.md)
      skill="${file#plugins/development-system/skills/}"
      skill_names+=("${skill%%/*}")
      ;;
    .agents/plugins/pi-support.json | plugins/development-system/package.json | plugins/development-system/extensions/* | plugins/development-system/bin/development-system-pi | scripts/bootstrap-pi-package.sh | scripts/pi-package-canary.mjs | scripts/validate-pi-package.mjs | scripts/evals/pi-provider.mjs | scripts/evals/prepare-pi-home.mjs | scripts/evals/provider-compositions.mjs | scripts/evals/generate-config.mjs)
      pi_package=1
      ;;
  esac
  case "$file" in
    plugins/development-system/extensions/development-system/core/guards.ts | plugins/development-system/extensions/development-system/adapters/ci-hold.ts)
      pi_guards=1
      ;;
    plugins/development-system/extensions/development-system/index.ts | plugins/development-system/extensions/development-system/adapters/logical-workspace.ts | plugins/development-system/extensions/development-system/adapters/worktrees.ts | plugins/development-system/extensions/development-system/core/worktrees.ts | plugins/development-system/skills/worktrees/SKILL.md | plugins/development-system/skills/delivery/SKILL.md | scripts/evals/run-pi-worktree-tui-scenario.mjs)
      pi_worktree_tui=1
      ;;
    plugins/development-system/extensions/development-system/core/goal.ts | plugins/development-system/extensions/development-system/adapters/goal-mode.ts)
      pi_goal=1
      ;;
    scripts/evals/run-pi-guard-scenarios.mjs)
      pi_guards=1
      pi_goal=1
      ;;
  esac
done

ran=0
if [ "${#skill_names[@]}" -gt 0 ]; then
  skills_json="$(printf '%s\n' "${skill_names[@]}" | sort -u | jq -Rsc 'split("\n") | map(select(length > 0))')"
  mapfile -t case_ids < <(jq -r --argjson skills "$skills_json" '
    .[] | select(any(.skills[]?; . as $skill | $skills | index($skill))) | .case_id
  ' evals/fixtures/agentic-systems-engineering/cases.json)
  if [ "${#case_ids[@]}" -gt 0 ]; then
    case_filter="^($(IFS='|'; echo "${case_ids[*]}"))$"
    echo "eval scope: shared skills [$(IFS=,; echo "${skill_names[*]}")] -> cases [$(( ${#case_ids[@]} ))], all supported harnesses, one sample"
    if [ "$dry_run" -eq 0 ]; then
      EVAL_CASE_FILTER="$case_filter" EVAL_SAMPLES=1 scripts/evals/run.sh
    fi
    ran=1
  else
    echo "eval scope: changed shared skills have no mapped provider-backed behavior cases"
  fi
fi

if [ "$pi_package" -eq 1 ]; then
  echo "eval scope: Pi package/runtime -> installed development-system canary, one sample"
  if [ "$dry_run" -eq 0 ]; then
    EVAL_PROVIDER_FILTER=pi-openai-gpt-5.6-terra EVAL_SAMPLES=1 scripts/evals/run.sh --suite canary
  fi
  ran=1
fi

if [ "$pi_guards" -eq 1 ]; then
  echo "eval scope: Pi guard/runtime -> executable baseline, worktree, and delivery outcomes"
  if [ "$dry_run" -eq 0 ]; then
    node scripts/evals/run-pi-guard-scenarios.mjs --scenario guards
  fi
  ran=1
fi

if [ "$pi_worktree_tui" -eq 1 ]; then
  echo "eval scope: Pi logical workspace -> provider-backed real local-TUI trajectory"
  if [ "$dry_run" -eq 0 ]; then
    node scripts/evals/run-pi-worktree-tui-scenario.mjs --live-tool
  fi
  ran=1
fi

if [ "$pi_goal" -eq 1 ]; then
  echo "eval scope: Pi autonomous goal -> settled continuation and guarded completion outcome"
  if [ "$dry_run" -eq 0 ]; then
    node scripts/evals/run-pi-guard-scenarios.mjs --scenario goal
  fi
  ran=1
fi

if [ "$ran" -eq 0 ]; then
  echo "eval scope: changed files do not affect a mapped provider-backed behavior; no provider evals selected"
fi
