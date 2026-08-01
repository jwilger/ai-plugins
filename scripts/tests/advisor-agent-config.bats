#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  CHECK="$ROOT/scripts/check-advisor-agent-config.sh"
  TMPROOT="$(mktemp -d)"
}

teardown() {
  rm -rf "$TMPROOT"
}

copy_plugin() {
  cp -R "$ROOT/plugins/development-system" "$TMPROOT/development-system"
}

@test "Advisor is public, capability-selected, read-only, and xhigh" {
  run "$CHECK"

  [ "$status" -eq 0 ]
  [ "$output" = "advisor-agent-config: ok" ]
}

@test "Advisor check rejects a fixed Codex model" {
  copy_plugin
  sed -i '/^description =/a model = "gpt-5.6-sol"' \
    "$TMPROOT/development-system/agents/advisor.toml"

  run "$CHECK" "$TMPROOT/development-system"

  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-codex-advisor-route"* ]]
}

@test "Advisor check rejects reduced reasoning" {
  copy_plugin
  sed -i 's/model_reasoning_effort = "xhigh"/model_reasoning_effort = "high"/' \
    "$TMPROOT/development-system/agents/advisor.toml"

  run "$CHECK" "$TMPROOT/development-system"

  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-codex-advisor-route"* ]]
}

@test "Advisor check rejects missing proactive BDD routing" {
  copy_plugin
  sed -i 's/two or more dependent implementation steps/multiple implementation steps/g' \
    "$TMPROOT/development-system/skills/advisor/SKILL.md"

  run "$CHECK" "$TMPROOT/development-system"

  [ "$status" -ne 0 ]
  [[ "$output" == *"skill-must-trigger-for-multi-step-plans"* ]]
}

@test "Advisor check rejects the obsolete nested component" {
  copy_plugin
  mkdir -p "$TMPROOT/development-system/components/advisor"

  run "$CHECK" "$TMPROOT/development-system"

  [ "$status" -ne 0 ]
  [[ "$output" == *"nested-advisor-component-must-be-removed"* ]]
}
