#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  CHECK="$ROOT/scripts/check-advisor-agent-config.sh"
  TMPROOT="$(mktemp -d)"
}

teardown() {
  rm -rf "$TMPROOT"
}

@test "advisor agent pins GPT-5.6 Sol with high reasoning and no fallback" {
  run "$CHECK" "$ROOT/plugins/advisor"

  [ "$status" -eq 0 ]
  [ "$output" = "advisor-agent-config: ok" ]
}

@test "advisor agent check rejects a different model" {
  cp -R "$ROOT/plugins/advisor" "$TMPROOT/advisor"
  sed -i 's/model = "gpt-5.6-sol"/model = "gpt-5.6"/' \
    "$TMPROOT/advisor/agents/advisor.toml"

  run "$CHECK" "$TMPROOT/advisor"

  [ "$status" -ne 0 ]
  [[ "$output" == *"model-must-be-gpt-5.6-sol"* ]]
}

@test "advisor agent check rejects a silent default-agent fallback" {
  cp -R "$ROOT/plugins/advisor" "$TMPROOT/advisor"
  printf '\nUse `agent_type: default` when unavailable.\n' \
    >>"$TMPROOT/advisor/skills/advisor/SKILL.md"

  run "$CHECK" "$TMPROOT/advisor"

  [ "$status" -ne 0 ]
  [[ "$output" == *"skill-configures-default-agent-fallback"* ]]
}

@test "advisor agent check rejects a different custom-agent fallback" {
  cp -R "$ROOT/plugins/advisor" "$TMPROOT/advisor"
  printf '\nIf the advisor is unavailable, use the custom `explorer` agent.\n' \
    >>"$TMPROOT/advisor/skills/advisor/SKILL.md"

  run "$CHECK" "$TMPROOT/advisor"

  [ "$status" -ne 0 ]
  [[ "$output" == *"skill-configures-agent-fallback"* ]]
}

@test "advisor agent check rejects fallback synonyms" {
  cp -R "$ROOT/plugins/advisor" "$TMPROOT/advisor"
  printf '\nIf the custom agent cannot start, spawn the default agent.\n' \
    >>"$TMPROOT/advisor/skills/advisor/SKILL.md"

  run "$CHECK" "$TMPROOT/advisor"

  [ "$status" -ne 0 ]
  [[ "$output" == *"skill-contract-drift"* ]]
}

@test "advisor agent check rejects a skill-level reasoning override" {
  cp -R "$ROOT/plugins/advisor" "$TMPROOT/advisor"
  printf '\nUse `reasoning_effort: medium` for quick advice.\n' \
    >>"$TMPROOT/advisor/skills/advisor/SKILL.md"

  run "$CHECK" "$TMPROOT/advisor"

  [ "$status" -ne 0 ]
  [[ "$output" == *"skill-reports-unpinned-reasoning-effort"* ]]
}

@test "advisor agent check rejects a contradictory effort footer" {
  cp -R "$ROOT/plugins/advisor" "$TMPROOT/advisor"
  printf '\nfooter: `effort=medium; playbook=no; context=none checked`\n' \
    >>"$TMPROOT/advisor/skills/advisor/SKILL.md"

  run "$CHECK" "$TMPROOT/advisor"

  [ "$status" -ne 0 ]
  [[ "$output" == *"skill-reports-unpinned-reasoning-effort"* ]]
}

@test "advisor agent check rejects alternate non-high effort claims" {
  local claim
  local index=0

  for claim in \
    'Use `model_reasoning_effort: low` for short requests.' \
    'Use reasoning effort: medium for quick advice.' \
    'Use low reasoning when the question looks simple.' \
    'The reasoning effort should be medium for quick checks.' \
    'Reasoning effort is low for trivial requests.'; do
    index=$((index + 1))
    cp -R "$ROOT/plugins/advisor" "$TMPROOT/advisor-$index"
    printf '\n%s\n' "$claim" \
      >>"$TMPROOT/advisor-$index/skills/advisor/SKILL.md"

    run "$CHECK" "$TMPROOT/advisor-$index"

    [ "$status" -ne 0 ]
    [[ "$output" == *"skill-reports-unpinned-reasoning-effort"* ]]
  done
}
