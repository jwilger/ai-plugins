#!/usr/bin/env bash
set -euo pipefail

root="$(cd "$(dirname "${BASH_SOURCE[0]}")/.." && pwd)"
sandbox="$(mktemp -d)"
trap 'rm -rf -- "$sandbox"' EXIT

git -C "$sandbox" init -q --initial-branch=main
git -C "$sandbox" config user.name "Formula Check"
git -C "$sandbox" config user.email "formula-check@example.invalid"
printf 'fixture\n' >"$sandbox/README.md"
git -C "$sandbox" add README.md
git -C "$sandbox" commit -qm "test: initialize formula fixture"
(
  cd "$sandbox"
  bd init --quiet --non-interactive --skip-agents --skip-hooks --prefix formula
)
mkdir -p "$sandbox/.beads/formulas"
cp "$root"/plugins/development-system/formulas/*.formula.toml \
  "$sandbox/.beads/formulas/"

for formula in "$root"/plugins/development-system/formulas/*.formula.toml; do
  name="$(basename "$formula" .formula.toml)"
  (
    cd "$sandbox"
    bd formula show "$name" --json >/dev/null
    bd cook "$name" --json >/dev/null
  )
done

(
  cd "$sandbox"
  result="$(
    bd mol pour development-change-direct \
      --var work_item="Formula integration fixture" \
      --var ci_workflow=ci.yml \
      --json
  )"
  work_item="$(jq -r '.new_epic_id' <<<"$result")"
  bd ready --mol "$work_item" --json | jq -e \
    '.steps | any(.issue.title == "Preflight: Formula integration fixture")' \
    >/dev/null
)

printf 'development_system.beads_formulas_valid count=%s\n' \
  "$(find "$root/plugins/development-system/formulas" -name '*.formula.toml' | wc -l)"
