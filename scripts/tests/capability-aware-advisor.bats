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

  run python3 - "$PLUGIN/skills/advisor/SKILL.md" <<'PY'
import pathlib
import sys

text = " ".join(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").split())
for phrase in (
    "On Claude Code, launch the exact named public `advisor` Agent",
    "inspect the exposed `spawn_agent` contract before invoking it",
    '`fork_turns: "none"`',
    "the explicitly selected highest-capability model, and `xhigh` reasoning effort",
    "block only when a required input control is absent",
    "the tool rejects the explicit controls or launch fails",
    "Treat `task_name` only as an operational label",
    "lack of a per-spawn read-only sandbox control is not itself a route failure",
    "Accept the route only after the child reaches terminal completion",
    "a parent-authored substitute are not Advisor results",
):
    assert phrase in text, phrase
PY
  [ "$status" -eq 0 ]
}

@test "Codex routes trust accepted explicit spawn controls without requiring echoed effective settings" {
  for skill in advisor content-authoring sharpen-plan; do
    run grep -F "Treat accepted explicit Codex spawn controls as authoritative" \
      "$PLUGIN/skills/$skill/SKILL.md"
    [ "$status" -eq 0 ]

    run grep -F "Do not require the spawn result to echo effective configuration" \
      "$PLUGIN/skills/$skill/SKILL.md"
    [ "$status" -eq 0 ]

    run grep -F "failed or empty child result" "$PLUGIN/skills/$skill/SKILL.md"
    [ "$status" -eq 0 ]

    run grep -F "cannot verify required Codex spawn_agent evidence fields" \
      "$PLUGIN/skills/$skill/SKILL.md"
    [ "$status" -ne 0 ]
  done
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

@test "Advisor delegated-dispatch fixture requires exact Claude and Codex evidence contracts" {
  run jq -e '
    map(select(.case_id == "advisor-installed-delegated-dispatch"))
    | length == 1
      and .[0].plugins == ["development-system"]
      and .[0].skills == ["advisor"]
      and .[0].dispatchEvidence == {
        "skill": "advisor",
        "claude": {
          "agent": "advisor"
        },
        "codex": {
          "taskName": "advisor",
          "model": "gpt-5.6-sol",
          "reasoningEffort": "xhigh",
          "forkTurns": "none"
        }
      }
      and .[0].coverage.kinds == [
        "core-behavior",
        "baseline-ablation"
      ]
      and .[0].valueGate == {
        "mode": "standard",
        "baselineLiftThreshold": 0.1
      }
      and .[0].minPassRate == 1
      and (.[0].semanticRubric | contains("completed foreground Advisor result"))
      and (.[0].semanticRubric | contains("empty wait"))
      and (.[0].semanticRubric | contains("self-authored prose"))
      and (.[0].semanticRubric | contains("visible capability block"))
  ' "$ROOT/evals/fixtures/behavior/development-system/cases.json"
  [ "$status" -eq 0 ]
}
