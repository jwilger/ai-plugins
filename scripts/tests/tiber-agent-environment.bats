#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  export CODEX_HOME="$BATS_TEST_TMPDIR/codex"
  VERSION="$(jq -r .version "$ROOT/plugins/tiber/plugin.json")"
  INSTALLED="$CODEX_HOME/plugins/cache/ai-plugins/tiber/$VERSION"
  DATA="$BATS_TEST_TMPDIR/plugin-data"
  mkdir -p "$INSTALLED/scripts" "$INSTALLED/bin" "$DATA"
  cp "$ROOT/plugins/tiber/plugin.json" "$INSTALLED/plugin.json"
  printf '#!/bin/sh\nexit 0\n' >"$INSTALLED/bin/tiber"
  chmod +x "$INSTALLED/bin/tiber"
  if [ -f "$ROOT/plugins/tiber/scripts/configure-codex-mcp.py" ]; then
    cp "$ROOT/plugins/tiber/scripts/configure-codex-mcp.py" "$INSTALLED/scripts/"
  fi
}

repair() {
  python3 "$INSTALLED/scripts/configure-codex-mcp.py" --plugin-data "$DATA" "$@"
}

@test "shared repair forwards by name, preserves other config, and converges without a session socket" {
  printf 'model = "example"\n[mcp_servers.other]\ncommand = "/trusted/other"\n' >"$CODEX_HOME/config.toml"
  export SSH_AUTH_SOCK=/private/session-one
  run repair --apply
  [ "$status" -eq 0 ]
  cp "$CODEX_HOME/config.toml" "$BATS_TEST_TMPDIR/first.toml"
  export SSH_AUTH_SOCK=/private/session-two
  run repair --apply
  [ "$status" -eq 0 ]
  cmp "$CODEX_HOME/config.toml" "$BATS_TEST_TMPDIR/first.toml"
  python3 - "$CODEX_HOME/config.toml" "$INSTALLED" "$DATA" <<'PY'
import pathlib,sys,tomllib
raw=pathlib.Path(sys.argv[1]).read_text()
c=tomllib.loads(raw)
s=c['mcp_servers']['tiber']
assert c['model']=='example' and c['mcp_servers']['other']['command']=='/trusted/other'
assert s['command']==sys.argv[2]+'/bin/tiber'
assert s['args']==['mcp','stdio'] and s['env_vars']==['SSH_AUTH_SOCK']
assert s['env']['PLUGIN_DATA']==sys.argv[3]
assert s['env']['TIBER_AGENT_FORWARDING']=='current-session'
assert 'session-one' not in raw and 'session-two' not in raw
assert pathlib.Path(sys.argv[1]).stat().st_mode & 0o777 == 0o600
PY
}

@test "repair leaves unmanaged direct registration intact and offers credential-safe recovery" {
  printf '[mcp_servers.tiber]\ncommand = "/trusted/custom"\n' >"$CODEX_HOME/config.toml"
  cp "$CODEX_HOME/config.toml" "$BATS_TEST_TMPDIR/before"
  run repair --apply
  [ "$status" -ne 0 ]
  [[ "$output" == *'unmanaged_registration'* ]]
  cmp "$CODEX_HOME/config.toml" "$BATS_TEST_TMPDIR/before"
}

@test "check does not write and managed repair updates an upgraded installed launcher" {
  run repair
  [ "$status" -ne 0 ]
  [ ! -f "$CODEX_HOME/config.toml" ]
  run repair --apply
  [ "$status" -eq 0 ]
  sed -i 's@/bin/tiber@/old-version/bin/tiber@' "$CODEX_HOME/config.toml"
  run repair --apply
  [ "$status" -eq 0 ]
  run repair
  [ "$status" -eq 0 ]
  ! rg -q old-version "$CODEX_HOME/config.toml"
}

@test "current-session forwarding never resurrects a saved agent when the parent has none" {
  mkdir -p "$DATA/ai-plugins/tiber/$VERSION/linux-x86_64"
  local binary="$DATA/ai-plugins/tiber/$VERSION/linux-x86_64/tiber"
  printf '#!/bin/sh\nprintf "agent=%%s\\n" "${SSH_AUTH_SOCK:-absent}"\n' >"$binary"
  chmod +x "$binary"
  printf '%s\n' "$VERSION" >"${binary%/*}/.plugin-version"
  python3 -c 'import socket,sys; s=socket.socket(socket.AF_UNIX); s.bind(sys.argv[1])' "$BATS_TEST_TMPDIR/old.sock"
  printf '%s\n' "$BATS_TEST_TMPDIR/old.sock" >"$DATA/signing-agent-socket"
  run env -u SSH_AUTH_SOCK PLUGIN_DATA="$DATA" TIBER_AGENT_FORWARDING=current-session "$ROOT/plugins/tiber/bin/tiber" --help
  [ "$status" -eq 0 ]
  [ "$output" = agent=absent ]
  run env SSH_AUTH_SOCK="$BATS_TEST_TMPDIR/new.sock" PLUGIN_DATA="$DATA" TIBER_AGENT_FORWARDING=current-session "$ROOT/plugins/tiber/bin/tiber" --help
  [ "$status" -eq 0 ]
  [ "$output" = "agent=$BATS_TEST_TMPDIR/new.sock" ]
}

@test "repair reports missing plugin data safely when a direct registration has null environment" {
  mkdir -p "$BATS_TEST_TMPDIR/bin"
  printf '#!/bin/sh\nprintf '\''[{"name":"tiber","transport":{"env":null}}]\\n'\''\n' >"$BATS_TEST_TMPDIR/bin/codex"
  chmod +x "$BATS_TEST_TMPDIR/bin/codex"
  run env PATH="$BATS_TEST_TMPDIR/bin:$PATH" python3 "$INSTALLED/scripts/configure-codex-mcp.py" --apply
  [ "$status" -ne 0 ]
  [[ "$output" == *'reason=plugin_data_unavailable'* ]]
  [[ "$output" != *'Traceback'* ]]
  [ ! -f "$CODEX_HOME/config.toml" ]
}
