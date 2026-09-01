#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  CHECK="$ROOT/scripts/check-model-routing-config.sh"
  TMPROOT="$(mktemp -d)"
  PLUGIN="$TMPROOT/development-discipline"
  mkdir -p "$PLUGIN/agents"
}

teardown() {
  rm -rf "$TMPROOT"
}

write_codex_agent() {
  local name="$1"
  local model="$2"
  local effort="$3"
  local sandbox="$4"

  cat >"$PLUGIN/agents/$name.toml" <<EOF
name = "model-routing-$name"
description = "Fixture route."
model = "$model"
model_reasoning_effort = "$effort"
sandbox_mode = "$sandbox"
developer_instructions = "Return the requested result."
EOF
}

write_valid_routes() {
  write_codex_agent bounded-helper gpt-5.6-luna low read-only
  write_codex_agent substantive-worker gpt-5.6-terra medium read-only
  write_codex_agent strong-reviewer gpt-5.6-sol high read-only
  write_codex_agent strong-worker gpt-5.6-sol high read-only
}

@test "model routing config reports exact task-local Codex routes" {
  write_valid_routes

  run "$CHECK" "$PLUGIN"

  [ "$status" -eq 0 ]
  [ "$(jq -c . <<<"$output")" = '{"codex":{"bounded-helper":{"model":"gpt-5.6-luna","reasoning":"low","sandbox":"read-only"},"substantive-worker":{"model":"gpt-5.6-terra","reasoning":"medium","sandbox":"read-only"},"strong-reviewer":{"model":"gpt-5.6-sol","reasoning":"high","sandbox":"read-only"},"strong-worker":{"model":"gpt-5.6-sol","reasoning":"high","sandbox":"read-only"}}}' ]
}

@test "model routing config rejects wrong values in every material route field" {
  local mutation

  for mutation in \
    'bounded-helper.toml|model = "gpt-5.6-luna"|model = "gpt-5.6-terra"' \
    'bounded-helper.toml|model_reasoning_effort = "low"|model_reasoning_effort = "high"' \
    'bounded-helper.toml|sandbox_mode = "read-only"|sandbox_mode = "workspace-write"'; do
    write_valid_routes
    IFS='|' read -r file expected replacement <<<"$mutation"
    sed -i "s|$expected|$replacement|" "$PLUGIN/agents/$file"

    run "$CHECK" "$PLUGIN"

    [ "$status" -ne 0 ]
    rm -rf "$PLUGIN"
    mkdir -p "$PLUGIN/agents"
  done
}

@test "model routing config rejects missing values in every material route field" {
  local mutation

  for mutation in \
    'bounded-helper.toml|model = "gpt-5.6-luna"' \
    'bounded-helper.toml|model_reasoning_effort = "low"' \
    'bounded-helper.toml|sandbox_mode = "read-only"'; do
    write_valid_routes
    IFS='|' read -r file line <<<"$mutation"
    sed -i "/^$line\$/d" "$PLUGIN/agents/$file"

    run "$CHECK" "$PLUGIN"

    [ "$status" -ne 0 ]
    rm -rf "$PLUGIN"
    mkdir -p "$PLUGIN/agents"
  done
}

@test "model routing config rejects duplicated values in every material route field" {
  local mutation

  for mutation in \
    'bounded-helper.toml|model = "gpt-5.6-luna"' \
    'bounded-helper.toml|model_reasoning_effort = "low"' \
    'bounded-helper.toml|sandbox_mode = "read-only"'; do
    write_valid_routes
    IFS='|' read -r file line <<<"$mutation"
    sed -i "/^$line\$/a\\$line" "$PLUGIN/agents/$file"

    run "$CHECK" "$PLUGIN"

    [ "$status" -ne 0 ]
    rm -rf "$PLUGIN"
    mkdir -p "$PLUGIN/agents"
  done
}

@test "model routing config rejects duplicate keys with alternate valid syntax" {
  write_valid_routes
  printf '%s\n' "model='gpt-5.6-terra'" >>"$PLUGIN/agents/bounded-helper.toml"

  run "$CHECK" "$PLUGIN"

  [ "$status" -ne 0 ]
  [[ "$output" == *"bounded-helper.toml:model"* ]]

}

@test "model routing config ignores protected-key text outside root configuration" {
  write_valid_routes
  cat >"$PLUGIN/agents/bounded-helper.toml" <<'EOF'
name = "model-routing-bounded-helper"
description = "Fixture route."
model = "gpt-5.6-luna"
model_reasoning_effort = "low"
sandbox_mode = "read-only"
developer_instructions = """
Explain this example when asked:
model = "gpt-5.6-terra"
"""
EOF
  run "$CHECK" "$PLUGIN"

  [ "$status" -eq 0 ]
}

@test "model routing config cannot hide duplicate TOML keys behind comment markers" {
  write_valid_routes
  cat >>"$PLUGIN/agents/bounded-helper.toml" <<'EOF'
# """
model='gpt-5.6-terra'
# """
EOF

  run "$CHECK" "$PLUGIN"

  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-toml-field: "*"bounded-helper.toml:model"* ]]
}
