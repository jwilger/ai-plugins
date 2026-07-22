#!/usr/bin/env bash
set -euo pipefail

plugin_root="${1:-"$(cd "$(dirname "${BASH_SOURCE[0]}")/../plugins/development-discipline" && pwd)"}"
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

yaml_value() {
  local file="$1"
  local key="$2"
  local lines

  [ -f "$file" ] || fail "missing-agent: $file"
  [ "$(sed -n '1p' "$file")" = '---' ] ||
    fail "$file:frontmatter-must-start-on-first-line"
  awk 'NR > 1 && $0 == "---" { closed = 1; exit } END { exit !closed }' "$file" ||
    fail "$file:frontmatter-must-close"
  lines="$(awk -v key="$key" '
    $0 == "---" {
      boundaries++
      if (boundaries == 1) {
        in_frontmatter = 1
        next
      }
      if (boundaries == 2) {
        exit
      }
    }
    in_frontmatter && $0 ~ ("^" key "[[:space:]]*:") { print }
  ' "$file")"
  [ "$(printf '%s\n' "$lines" | grep -c .)" -eq 1 ] ||
    fail "field-count-invalid: $file:$key"
  printf '%s\n' "$lines" | sed -E 's/^[^:]+: //'
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

check_claude() {
  local route="$1"
  local model="$2"
  local tools="$3"
  local file="$agents/$route.md"

  require_equal "$(yaml_value "$file" model)" "$model" "$route.claude.model"
  require_equal "$(yaml_value "$file" tools)" "$tools" "$route.claude.tools"
}

check_codex bounded-helper gpt-5.6-luna low read-only
check_codex substantive-worker gpt-5.6-terra medium workspace-write
check_codex strong-reviewer gpt-5.6-sol high read-only
check_claude bounded-helper haiku Read,Grep,Glob,Bash
check_claude substantive-worker sonnet Read,Grep,Glob,Bash,Write,Edit
check_claude strong-reviewer opus Read,Grep,Glob,Bash

jq -cn '{
  codex: {
    "bounded-helper": {model: "gpt-5.6-luna", reasoning: "low", sandbox: "read-only"},
    "substantive-worker": {model: "gpt-5.6-terra", reasoning: "medium", sandbox: "workspace-write"},
    "strong-reviewer": {model: "gpt-5.6-sol", reasoning: "high", sandbox: "read-only"}
  },
  claude: {
    "bounded-helper": {model: "haiku", tools: "Read,Grep,Glob,Bash"},
    "substantive-worker": {model: "sonnet", tools: "Read,Grep,Glob,Bash,Write,Edit"},
    "strong-reviewer": {model: "opus", tools: "Read,Grep,Glob,Bash"}
  }
}'
