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
    "highest-capability eligible agent",
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

  run python3 - "$skill" <<'PY'
import pathlib
import sys

text = " ".join(pathlib.Path(sys.argv[1]).read_text(encoding="utf-8").split())
for phrase in (
    "On Claude Code, launch the exact named public `strong-worker` Agent",
    'generic `spawn_agent` mechanism with `fork_turns: "none"`',
    "the explicitly selected model, and high reasoning effort",
    "`task_name` is only an operational label",
    "block only when a required `model`, `reasoning_effort`, or `fork_turns` input control is absent",
    "the tool rejects the explicit controls or launch fails",
    "The parent must not draft, outline, exemplify, template, partially create, or otherwise substitute",
    "Accept the route only after the child reaches terminal completion",
    "a parent-authored substitute are not authored results",
):
    assert phrase in text, phrase
PY
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

@test "content-authoring delegated-dispatch fixture requires exact Claude and Codex evidence contracts" {
  run jq -e '
    map(select(.case_id == "content-authoring-installed-delegated-dispatch"))
    | length == 1
      and .[0].plugins == ["development-system"]
      and .[0].skills == ["content-authoring"]
      and .[0].dispatchEvidence == {
        "skill": "content-authoring",
        "claude": {
          "agent": "strong-worker"
        },
        "codex": {
          "taskName": "content_author",
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
      and (.[0].semanticRubric | contains("finished quick-start card itself"))
      and (.[0].semanticRubric | contains("empty wait"))
      and (.[0].semanticRubric | contains("self-authored copy"))
      and (.[0].semanticRubric | contains("visible capability block"))
  ' "$ROOT/evals/fixtures/behavior/development-system/cases.json"
  [ "$status" -eq 0 ]
}
