#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  PLUGIN="$ROOT/plugins/development-system"
}

@test "given an uncovered multi-step plan the public Advisor uses the strongest available model at xhigh" {
  [ -f "$PLUGIN/skills/advisor/SKILL.md" ]
  [ -f "$PLUGIN/agents/advisor.toml" ]
  [ ! -e "$PLUGIN/components/advisor" ]

  run python3 - "$PLUGIN/agents/advisor.toml" <<'PY'
import pathlib
import sys
import tomllib

agent = tomllib.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert "model" not in agent, "Advisor must receive the strongest advertised model at dispatch time"
assert agent["model_reasoning_effort"] == "xhigh"
assert agent["sandbox_mode"] == "read-only"
PY
  [ "$status" -eq 0 ]

  run grep -F "highest-capability eligible model" "$PLUGIN/skills/advisor/SKILL.md"
  [ "$status" -eq 0 ]
  run grep -F "existing executable BDD-style scenario" "$PLUGIN/skills/advisor/SKILL.md"
  [ "$status" -eq 0 ]
  run grep -F "two or more dependent implementation steps" "$PLUGIN/skills/advisor/SKILL.md"
  [ "$status" -eq 0 ]
}

@test "given future model upgrades strong routes defer model selection to current harness capability metadata" {
  for route in strong-reviewer strong-worker; do
    [ -f "$PLUGIN/agents/$route.toml" ]
    run python3 - "$PLUGIN/agents/$route.toml" <<'PY'
import pathlib
import sys
import tomllib

agent = tomllib.loads(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8"))
assert "model" not in agent
assert agent["model_reasoning_effort"] == "high"
PY
    [ "$status" -eq 0 ]

    run grep -F "model: opus" "$PLUGIN/agents/$route.md"
    [ "$status" -eq 0 ]
    run grep -F "effort: high" "$PLUGIN/agents/$route.md"
    [ "$status" -eq 0 ]
  done

  run grep -F "highest-capability eligible model" \
    "$PLUGIN/components/development-discipline/skills/model-routing/SKILL.md"
  [ "$status" -eq 0 ]
  run grep -F "Never infer capability from" \
    "$PLUGIN/components/development-discipline/skills/model-routing/SKILL.md"
  [ "$status" -eq 0 ]
}

@test "given implementation planning the workflow applies the Advisor BDD coverage gate before finalization" {
  local workflow="$PLUGIN/skills/development-workflow/SKILL.md"
  local detailed="$PLUGIN/components/development-discipline/skills/development-workflow/SKILL.md"
  local preflight="$PLUGIN/components/development-discipline/skills/change-preflight/SKILL.md"

  for file in "$workflow" "$detailed" "$preflight"; do
    run grep -F "two or more dependent implementation steps" "$file"
    [ "$status" -eq 0 ]
    run grep -F "existing executable BDD-style" "$file"
    [ "$status" -eq 0 ]
    run grep -F "material failure boundary" "$file"
    [ "$status" -eq 0 ]
  done
}
