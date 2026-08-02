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
  shared skill prose     only mapped cases with an explicit unresolved stochastic question
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
for file in "${changed[@]}"; do
  case "$file" in
    plugins/development-system/skills/*/SKILL.md | plugins/development-system/skills/*/references/*)
      skill="${file#plugins/development-system/skills/}"
      skill_names+=("${skill%%/*}")
      ;;
    plugins/development-system/components/*/skills/*/SKILL.md | plugins/development-system/components/*/skills/*/references/*)
      component_path="${file#plugins/development-system/components/}"
      component="${component_path%%/*}"
      component_skill_path="${component_path#*/skills/}"
      component_skill="${component_skill_path%%/*}"
      case "$component/$component_skill" in
        agentic-systems-engineering/*)
          skill_names+=("agentic-systems")
          ;;
        babysit-pr/*)
          skill_names+=("delivery")
          ;;
        development-discipline/delivery-workflow | development-discipline/ci-failure-follow-up | development-discipline/final-review | development-discipline/rationale-commit-messages | development-discipline/verification-before-completion)
          skill_names+=("delivery" "development-workflow")
          ;;
        development-discipline/*)
          skill_names+=("development-workflow")
          ;;
        engineering-standards/*)
          skill_names+=("engineering-standards")
          ;;
        eval-case-reporter/*)
          skill_names+=("eval-case-reporting")
          ;;
      esac
      ;;
  esac
done

ran=0
if [ "${#skill_names[@]}" -gt 0 ]; then
  mapfile -t skill_names < <(printf '%s\n' "${skill_names[@]}" | LC_ALL=C sort -u)
  skills_json="$(printf '%s\n' "${skill_names[@]}" | jq -Rsc 'split("\n") | map(select(length > 0))')"

  behavior_fixture_files=()
  if [ -d evals/fixtures/behavior ]; then
    mapfile -t behavior_fixture_files < <(
      find evals/fixtures/behavior -type f -name '*.json' -print | LC_ALL=C sort
    )
  fi

  case_ids=()
  if [ "${#behavior_fixture_files[@]}" -gt 0 ]; then
    mapfile -t case_ids < <(
      jq -r --argjson skills "$skills_json" '
        (if type == "array" then . else .cases end)[]?
        | select(
            any(.plugins[]?; . == "development-system")
            and any(.skills[]?; . as $skill | $skills | index($skill))
            and .providerEval.deterministicVerificationFullyDecides == false
            and (.providerEval.unresolvedStochasticQuestion | type == "string" and length > 0)
            and (.providerEval.deterministicInsufficiency | type == "string" and length > 0)
          )
        | .case_id
      ' "${behavior_fixture_files[@]}" | LC_ALL=C sort -u
    )
  fi
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

if [ "$ran" -eq 0 ]; then
  echo "eval scope: changed files do not affect a mapped provider-backed behavior; no provider evals selected"
fi
