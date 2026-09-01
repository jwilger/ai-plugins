#!/usr/bin/env bash
set -euo pipefail

plugin_root="${1:-"$(cd "$(dirname "${BASH_SOURCE[0]}")/../plugins/development-system/components/development-discipline" && pwd)"}"
agents="$plugin_root/agents"

fail() {
  echo "model-routing-config: $*" >&2
  exit 1
}

quoted_value() {
  local file="$1"
  local key="$2"
  local value

  [ -f "$file" ] || fail "missing-agent: $file"
  if ! value="$(python3 - "$file" "$key" <<'PY'
import pathlib
import sys
import tomllib

path = pathlib.Path(sys.argv[1])
key = sys.argv[2]

try:
    document = tomllib.loads(path.read_text(encoding="utf-8"))
except (OSError, UnicodeError, tomllib.TOMLDecodeError):
    raise SystemExit(1)

value = document.get(key)
if not isinstance(value, str):
    raise SystemExit(1)

print(value)
PY
  )"; then
    fail "invalid-toml-field: $file:$key"
  fi
  printf '%s\n' "$value"
}

require_equal() {
  local actual="$1"
  local expected="$2"
  local label="$3"

  [ "$actual" = "$expected" ] ||
    fail "$label expected=$expected actual=$actual"
}

check_codex() {
  local route="$1"
  local model="$2"
  local reasoning="$3"
  local sandbox="$4"
  local file="$agents/$route.toml"

  require_equal "$(quoted_value "$file" model)" "$model" "$route.codex.model"
  require_equal "$(quoted_value "$file" model_reasoning_effort)" "$reasoning" "$route.codex.reasoning"
  require_equal "$(quoted_value "$file" sandbox_mode)" "$sandbox" "$route.codex.sandbox"
}

check_codex bounded-helper gpt-5.6-luna low read-only
check_codex substantive-worker gpt-5.6-terra medium read-only
check_codex strong-reviewer gpt-5.6-sol high read-only
check_codex strong-worker gpt-5.6-sol high read-only

jq -cn '{
  codex: {
    "bounded-helper": {model: "gpt-5.6-luna", reasoning: "low", sandbox: "read-only"},
    "substantive-worker": {model: "gpt-5.6-terra", reasoning: "medium", sandbox: "read-only"},
    "strong-reviewer": {model: "gpt-5.6-sol", reasoning: "high", sandbox: "read-only"},
    "strong-worker": {model: "gpt-5.6-sol", reasoning: "high", sandbox: "read-only"}
  }
}'
