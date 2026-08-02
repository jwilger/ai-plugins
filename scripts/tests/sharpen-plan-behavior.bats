#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  PLUGIN="$ROOT/plugins/development-system"
}

@test "sharpen-plan applies one plan-level improvement per approved pass" {
  local skill="$PLUGIN/skills/sharpen-plan/SKILL.md"
  local metadata="$PLUGIN/skills/sharpen-plan/agents/openai.yaml"

  [ -f "$skill" ]
  [ -f "$metadata" ]

  run python3 - "$skill" <<'PY'
import pathlib
import sys

raw = pathlib.Path(sys.argv[1]).read_text(encoding="utf-8")
frontmatter = raw.split("---", 2)[1].strip().splitlines()
assert [line.split(":", 1)[0] for line in frontmatter] == ["name", "description"]

text = " ".join(raw.split())
for phrase in (
    "exactly one highest-leverage assumption or plan-level specification per pass",
    "implementation minutiae",
    "genuine fork",
    "active plan artifact or conversational plan state",
    "harness-native approval",
    "Repeat only after approval",
    "diminishing returns",
    "Plan Mode",
    "read-only",
    "Do not invoke Advisor",
):
    assert phrase in text, phrase

assert "Stop-hook" not in raw
assert "/goal" not in raw
PY
  [ "$status" -eq 0 ]

  run grep -F 'allow_implicit_invocation: true' "$metadata"
  [ "$status" -eq 0 ]

  run python3 - "$skill" <<'PY'
import pathlib
import sys

text = " ".join(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").split())
for phrase in (
    "On Claude Code, launch the exact named public read-only `strong-reviewer`",
    "inspect the exposed `spawn_agent` contract before invoking it",
    '`fork_turns: "none"`',
    "the explicitly selected highest-capability model, and high reasoning effort",
    "block only when a required input control is absent",
    "the tool rejects the explicit controls or launch fails",
    "Treat `task_name` only as an operational label",
    "lack of a per-spawn read-only sandbox control is not itself a route failure",
    "Do not substitute a weaker or default route, make the judgment in the parent",
    "wait for terminal child completion",
    "parent-authored substitute is not a sharpening result",
):
    assert phrase in text, phrase
PY
  [ "$status" -eq 0 ]
}

@test "sharpen-plan is public and has an all-five behavior fixture" {
  run grep -F -- '- `sharpen-plan`' "$PLUGIN/README.md"
  [ "$status" -eq 0 ]

  run jq -e '
    map(select(.case_id == "sharpen-plan-one-improvement-approved-loop"))
    | length == 1
      and .[0].skills == ["sharpen-plan"]
      and .[0].coverage.kinds == [
        "natural-trigger",
        "scope-boundary",
        "core-behavior",
        "adversarial-safety",
        "baseline-ablation"
      ]
  ' "$ROOT/evals/fixtures/behavior/development-system/cases.json"
  [ "$status" -eq 0 ]
}

@test "sharpen-plan delegated-dispatch fixture requires exact Claude and Codex evidence contracts" {
  run jq -e '
    map(select(.case_id == "sharpen-plan-installed-delegated-dispatch"))
    | length == 1
      and .[0].plugins == ["development-system"]
      and .[0].skills == ["sharpen-plan"]
      and .[0].dispatchEvidence == {
        "skill": "sharpen-plan",
        "claude": {
          "agent": "strong-reviewer"
        },
        "codex": {
          "taskName": "sharpen_plan_author",
          "model": "gpt-5.6-sol",
          "reasoningEffort": "high",
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
      and (.[0].semanticRubric | contains("completed foreground reviewer result"))
      and (.[0].semanticRubric | contains("empty wait"))
      and (.[0].semanticRubric | contains("self-authored revision"))
      and (.[0].semanticRubric | contains("visible capability block"))
  ' "$ROOT/evals/fixtures/behavior/development-system/cases.json"
  [ "$status" -eq 0 ]
}
