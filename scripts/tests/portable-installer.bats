#!/usr/bin/env bats
setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  TMPROOT="$(mktemp -d /tmp/ai-plugins-installer.XXXXXX)"
  mkdir -p "$TMPROOT/bin"
  cat >"$TMPROOT/bin/codex" <<'SH'
#!/bin/sh
case "$*" in
  'plugin marketplace list --json') printf '%s\n' '{"marketplaces":[]}' ;;
  'plugin list --json'|'-C /tmp plugin list --json')
    if [ "${MOCK_INSTALLED:-}" = 1 ]; then
      printf '%s\n' '{"installed":[{"pluginId":"development-system@ai-plugins","name":"development-system","version":"10.0.0"},{"pluginId":"tiber@ai-plugins","name":"tiber","version":"1.0.0","installed":true,"enabled":true}],"available":[]}'
    else
      printf '%s\n' '{"installed":[],"available":[{"name":"github","pluginId":"github@curated"},{"name":"coderabbit","pluginId":"coderabbit@curated"}]}'
    fi
    ;;
  '-C /tmp mcp list --json') printf '[{"name":"development-discipline","transport":{"env":{"PLUGIN_ROOT":"/tmp/cache/development-system/10.0.0","PLUGIN_DATA":"%s"}}},{"name":"tiber","transport":{"env":{"PLUGIN_ROOT":"%s/plugins/cache/ai-plugins/tiber/1.0.0","PLUGIN_DATA":"%s"}}}]\n' "$MOCK_PLUGIN_DATA-development-system" "$CODEX_HOME" "$MOCK_PLUGIN_DATA" ;;
  *) printf '%s\n' "$*" >>"$CODEX_CALL_LOG" ;;
esac
SH
  chmod +x "$TMPROOT/bin/codex"
  export CODEX_CALL_LOG="$TMPROOT/calls"
  export CODEX_HOME="$TMPROOT/codex"
  local installed_root="$CODEX_HOME/plugins/cache/ai-plugins/tiber/1.0.0"
  mkdir -p "$installed_root/scripts" "$installed_root/bin"
  cp "$ROOT/plugins/tiber/scripts/"*.py "$installed_root/scripts/"
  printf '%s\n' '{"version":"1.0.0"}' >"$installed_root/plugin.json"
  printf '#!/bin/sh\nexit 0\n' >"$installed_root/bin/tiber"
  chmod +x "$installed_root/bin/tiber"
}
teardown() { rm -rf "$TMPROOT"; }

@test "dry run tracks main and prints optional selections without mutation" {
  run env PATH="$TMPROOT/bin:$PATH" sh "$ROOT/install.sh" --dry-run --with github,coderabbit
  [ "$status" -eq 0 ]
  [[ "$output" == *'codex plugin marketplace add jwilger/ai-plugins --ref main'* ]]
  [[ "$output" == *'codex plugin add development-system@ai-plugins'* ]]
  [[ "$output" == *'codex plugin add tiber@ai-plugins'* ]]
  [[ "$output" == *'codex plugin add github@curated'* ]]
  [[ "$output" == *'codex plugin add coderabbit@curated'* ]]
  [ ! -e "$CODEX_CALL_LOG" ]
}

@test "installer rejects an unrelated marketplace with the same name" {
  sed -i 's/{"marketplaces":\[\]}/{"marketplaces":[{"name":"ai-plugins","marketplaceSource":{"sourceType":"git","source":"https:\/\/example.org\/other.git"}}]}/' "$TMPROOT/bin/codex"
  run env PATH="$TMPROOT/bin:$PATH" sh "$ROOT/install.sh" --dry-run --with none
  [ "$status" -ne 0 ]
  [[ "$output" == *'marketplace points elsewhere'* ]]
}

@test "installer converges shared forwarding without persisting the host socket and upgrades idempotently" {
  export MOCK_INSTALLED=1
  export MOCK_PLUGIN_DATA="$TMPROOT/plugin-data"
  mkdir -p "$TMPROOT/plugin-data"
  python3 -c 'import socket,sys; s=socket.socket(socket.AF_UNIX); s.bind(sys.argv[1])' "$TMPROOT/agent.sock"
  run env PATH="$TMPROOT/bin:$PATH" SSH_AUTH_SOCK="$TMPROOT/agent.sock" \
    MOCK_INSTALLED=1 MOCK_PLUGIN_DATA="$MOCK_PLUGIN_DATA" \
    sh "$ROOT/install.sh" --with none
  [ "$status" -eq 0 ]
  [ ! -e "$MOCK_PLUGIN_DATA/signing-agent-socket" ]
  python3 - "$CODEX_HOME/config.toml" "$TMPROOT/agent.sock" <<'PYTEST'
import sys,tomllib
text=open(sys.argv[1]).read()
server=tomllib.loads(text)['mcp_servers']['tiber']
assert server['env_vars']==['SSH_AUTH_SOCK']
assert server['env']['TIBER_AGENT_FORWARDING']=='current-session'
assert sys.argv[2] not in text
PYTEST
  cp "$CODEX_HOME/config.toml" "$TMPROOT/before.toml"
  run env PATH="$TMPROOT/bin:$PATH" SSH_AUTH_SOCK="$TMPROOT/rotated.sock" \
    MOCK_INSTALLED=1 MOCK_PLUGIN_DATA="$MOCK_PLUGIN_DATA" \
    sh "$ROOT/install.sh" --with none
  [ "$status" -eq 0 ]
  cmp "$TMPROOT/before.toml" "$CODEX_HOME/config.toml"
}

@test "installer rejects missing tomllib before marketplace or plugin mutations" {
  export REAL_PYTHON="$(command -v python3)"
  cat >"$TMPROOT/bin/python3" <<'SH'
#!/bin/sh
case "$*" in
  '-c import tomllib'|*configure-codex-mcp.py*) exit 1 ;;
  *) exec "$REAL_PYTHON" "$@" ;;
esac
SH
  chmod +x "$TMPROOT/bin/python3"
  run env PATH="$TMPROOT/bin:$PATH" sh "$ROOT/install.sh" --with none
  [ "$status" -ne 0 ]
  [[ "$output" == *'Python 3.11 or later with tomllib is required'* ]]
  [ ! -e "$CODEX_CALL_LOG" ]
  [ ! -e "$CODEX_HOME/config.toml" ]
}
