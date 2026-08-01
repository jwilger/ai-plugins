#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  PLUGIN="$ROOT/plugins/development-system"
}

@test "substantive human-consumable content uses the capability-ranked writable author route" {
  local skill="$PLUGIN/skills/content-authoring/SKILL.md"
  local metadata="$PLUGIN/skills/content-authoring/agents/openai.yaml"

  [ -f "$skill" ]
  [ -f "$metadata" ]

  run python3 - "$skill" <<'PY'
import pathlib
import sys

text = " ".join(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").split())
for phrase in (
    "highest-capability eligible writable agent",
    "high effort",
    "prose, documentation, instructions, imagery, or UI/UX",
    "status updates, ticket metadata, commit messages, or mechanical summaries",
    "specialized image tool",
    "grants no additional authority",
):
    assert phrase in text, phrase
PY
  [ "$status" -eq 0 ]

  run grep -F 'allow_implicit_invocation: true' "$metadata"
  [ "$status" -eq 0 ]
}

@test "development workflow and model routing reserve substantive content for the author route" {
  run grep -F "content-authoring" "$PLUGIN/skills/development-workflow/SKILL.md"
  [ "$status" -eq 0 ]
  run grep -F "highest-capability eligible writable agent" \
    "$PLUGIN/components/development-discipline/skills/model-routing/SKILL.md"
  [ "$status" -eq 0 ]
}

@test "content-authoring behavior fixture declares all five root-skill coverage kinds" {
  run jq -e '
    map(select(.case_id == "content-authoring-capability-writable-route"))
    | length == 1
      and .[0].skills == ["content-authoring"]
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
