#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  CHECK="$ROOT/scripts/check-model-routing-config.sh"
  TMPROOT="$(mktemp -d)"
  PLUGIN="$TMPROOT/development-system"
  mkdir -p "$PLUGIN/agents"
  for name in bounded-helper substantive-worker strong-reviewer strong-worker; do
    cat >"$PLUGIN/agents/$name.toml" <<EOF_AGENT
name = "model-routing-$name"
description = "Fixture route."
sandbox_mode = "read-only"
developer_instructions = "Return the requested result."
EOF_AGENT
  done
  cat >"$PLUGIN/agents/advisor.toml" <<'EOF_AGENT'
name = "advisor"
description = "Fixture advisor."
sandbox_mode = "read-only"
developer_instructions = "Return the requested result."
EOF_AGENT
}

teardown() {
  rm -rf "$TMPROOT"
}

@test "agent definitions keep boundaries without hardcoded model routes" {
  run "$CHECK" "$PLUGIN"
  [ "$status" -eq 0 ]
  [ "$(jq -r '.codex["bounded-helper"].sandbox' <<<"$output")" = "read-only" ]
  [ "$(jq -r '.codex["strong-reviewer"].sandbox' <<<"$output")" = "read-only" ]
}

@test "agent definitions reject a hardcoded model or effort" {
  for key in model model_reasoning_effort reasoning_effort; do
    printf '%s = "pinned"\n' "$key" >>"$PLUGIN/agents/bounded-helper.toml"
    run "$CHECK" "$PLUGIN"
    [ "$status" -ne 0 ]
    [[ "$output" == *"hardcoded-route:"* ]]
    sed -i '$d' "$PLUGIN/agents/bounded-helper.toml"
  done
}

@test "agent definitions reject invalid TOML and changed sandbox" {
  printf '%s\n' 'sandbox_mode = "workspace-write"' >>"$PLUGIN/agents/strong-reviewer.toml"
  run "$CHECK" "$PLUGIN"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-agent:"* ]]
}
