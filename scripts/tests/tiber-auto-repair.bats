#!/usr/bin/env bats
setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  export CODEX_HOME="$BATS_TEST_TMPDIR/codex"
  export PLUGIN_DATA="$BATS_TEST_TMPDIR/data"
  export INVENTORY="$BATS_TEST_TMPDIR/inventory.json"
  export INVENTORY_AGENT="$BATS_TEST_TMPDIR/inventory-agent"
  mkdir -p "$PLUGIN_DATA" "$BATS_TEST_TMPDIR/bin"
  cat > "$BATS_TEST_TMPDIR/bin/codex" <<'SH'
#!/bin/sh
printf '%s' "${SSH_AUTH_SOCK:-absent}" > "$INVENTORY_AGENT"
cat "$INVENTORY"
SH
  chmod +x "$BATS_TEST_TMPDIR/bin/codex"
  export PATH="$BATS_TEST_TMPDIR/bin:$PATH"
  install_version 1.2.0
}
install_version() {
  INSTALLED="$CODEX_HOME/plugins/cache/ai-plugins/tiber/$1"
  mkdir -p "$INSTALLED/scripts" "$INSTALLED/bin"
  cp "$ROOT/plugins/tiber/scripts/"*.py "$INSTALLED/scripts/"
  printf '{\n  "version": "%s"\n}\n' "$1" > "$INSTALLED/plugin.json"
  printf '#!/bin/sh\nprintf "root=%%s agent=%%s\\n" "$PLUGIN_ROOT" "${SSH_AUTH_SOCK:-absent}"\n' > "$INSTALLED/bin/tiber"
  chmod +x "$INSTALLED/bin/tiber"
  printf '{"installed":[{"pluginId":"tiber@ai-plugins","version":"%s","installed":true,"enabled":true}]}' "$1" > "$INVENTORY"
}
repair() { python3 "$INSTALLED/scripts/configure-codex-mcp.py" --plugin-data "$PLUGIN_DATA" --apply; }
launch() {
  python3 - "$CODEX_HOME/config.toml" <<'PY'
import os,sys,tomllib
s=tomllib.load(open(sys.argv[1],'rb'))['mcp_servers']['tiber']
env=dict(os.environ,**s['env'])
os.execve(s['command'],[s['command'],*s['args']],env)
PY
}
@test "stable plugin dispatcher survives removed cache and forwards only to currently installed version" {
  export SSH_AUTH_SOCK=/synthetic/current-agent
  run repair
  [ "$status" -eq 0 ]
  cp "$CODEX_HOME/config.toml" "$BATS_TEST_TMPDIR/original"
  old="$INSTALLED"
  install_version 1.3.0-rc.1+fixture
  rm -rf "$old"
  run launch
  [ "$status" -eq 0 ]
  [ "$output" = "root=$INSTALLED agent=$SSH_AUTH_SOCK" ]
  [ "$(cat "$INVENTORY_AGENT")" = absent ]
  cmp "$CODEX_HOME/config.toml" "$BATS_TEST_TMPDIR/original"
  run repair
  [ "$status" -eq 0 ]
  [[ "$output" == *mcp_configuration_current* ]]
  cmp "$CODEX_HOME/config.toml" "$BATS_TEST_TMPDIR/original"
}
@test "dispatcher refuses disabled plugins without exposing source and allows absent agent" {
  run repair
  [ "$status" -eq 0 ]
  run env -u SSH_AUTH_SOCK bash -c 'python3 - "$CODEX_HOME/config.toml" <<'\''PY'\''
import os,sys,tomllib
s=tomllib.load(open(sys.argv[1],"rb"))["mcp_servers"]["tiber"]
os.execve(s["command"],[s["command"],*s["args"]],dict(os.environ,**s["env"]))
PY'
  [ "$status" -eq 0 ]
  [[ "$output" == *agent=absent ]]
  sed -i 's/"enabled":true/"enabled":false/' "$INVENTORY"
  run launch
  [ "$status" -ne 0 ]
  [[ "$output" == *plugin_not_enabled* ]]
  [[ "$output" != *"$BATS_TEST_TMPDIR"* ]]
}
@test "older installed helper cannot downgrade selected plugin configuration" {
  old="$INSTALLED"
  install_version 1.3.0
  run python3 "$old/scripts/configure-codex-mcp.py" --plugin-data "$PLUGIN_DATA" --apply
  [ "$status" -ne 0 ]
  [[ "$output" == *inactive_plugin_version* ]]
  [ ! -f "$CODEX_HOME/config.toml" ]
}
@test "installed portable MCP startup bootstraps once without changing CLI calls or MCP stdout" {
  cp "$ROOT/plugins/tiber/bin/tiber" "$INSTALLED/bin/tiber"
  mkdir -p "$INSTALLED/lib"
  cp "$ROOT/plugins/tiber/lib/installed-binary.sh" "$INSTALLED/lib/"
  version=1.2.0
  platform="$(uname -s | tr '[:upper:]' '[:lower:]')-$(uname -m)"
  binary_dir="$PLUGIN_DATA/ai-plugins/tiber/$version/$platform"
  mkdir -p "$binary_dir"
  printf '#!/bin/sh\nprintf "native-output\\n"\n' > "$binary_dir/tiber"
  chmod +x "$binary_dir/tiber"
  printf '%s\n' "$version" > "$binary_dir/.plugin-version"
  run env PLUGIN_ROOT="$INSTALLED" "$INSTALLED/bin/tiber" --help
  [ "$status" -eq 0 ]
  [ ! -f "$CODEX_HOME/config.toml" ]
  env PLUGIN_ROOT="$INSTALLED" "$INSTALLED/bin/tiber" mcp stdio > "$BATS_TEST_TMPDIR/stdout" 2> "$BATS_TEST_TMPDIR/stderr"
  [ "$(cat "$BATS_TEST_TMPDIR/stdout")" = native-output ]
  [ -f "$CODEX_HOME/config.toml" ]
  [[ "$(cat "$BATS_TEST_TMPDIR/stderr")" == *restart_Codex* ]]
}
@test "dispatcher pins Codex rather than accepting a hostile runtime PATH replacement" {
  run repair
  [ "$status" -eq 0 ]
  mkdir "$BATS_TEST_TMPDIR/hostile"
  printf '#!/bin/sh\nexit 77\n' > "$BATS_TEST_TMPDIR/hostile/codex"
  chmod +x "$BATS_TEST_TMPDIR/hostile/codex"
  export PATH="$BATS_TEST_TMPDIR/hostile:$PATH"
  run launch
  [ "$status" -eq 0 ]
  rm "$BATS_TEST_TMPDIR/bin/codex"
  run launch
  [ "$status" -ne 0 ]
  [[ "$output" == *codex_executable_unavailable* ]]
}
@test "dispatcher follows local selection and rejects malformed or symlink installed roots" {
  run repair
  [ "$status" -eq 0 ]
  install_version local
  run launch
  [ "$status" -eq 0 ]
  [[ "$output" == "root=$INSTALLED "* ]]
  mv "$INSTALLED" "$BATS_TEST_TMPDIR/moved"
  ln -s "$BATS_TEST_TMPDIR/moved" "$INSTALLED"
  run launch
  [ "$status" -ne 0 ]
  [[ "$output" == *installed_launcher_unavailable* ]]
  printf '{"installed":[{"pluginId":"tiber@ai-plugins","version":"../private","installed":true,"enabled":true}]}' > "$INVENTORY"
  run launch
  [ "$status" -ne 0 ]
  [[ "$output" == *invalid_installed_version* ]]
  [[ "$output" != *private* ]]
}
@test "malformed inventory fails without a traceback or source disclosure" {
  run repair
  [ "$status" -eq 0 ]
  printf '{"installed":["private-source-marker"]}' > "$INVENTORY"
  run launch
  [ "$status" -ne 0 ]
  [[ "$output" == *tiber.mcp_dispatch_failed*reason=inventory_unavailable* ]]
  [[ "$output" != *Traceback* ]]
  [[ "$output" != *private-source-marker* ]]
}
