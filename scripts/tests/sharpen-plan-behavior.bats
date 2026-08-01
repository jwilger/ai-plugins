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
