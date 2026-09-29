#!/usr/bin/env bats
setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  TMPROOT="$(mktemp -d /tmp/ai-plugins-installer.XXXXXX)"
  mkdir -p "$TMPROOT/bin"
  cat >"$TMPROOT/bin/codex" <<'SH'
#!/bin/sh
case "$*" in
  'plugin marketplace list --json') printf '%s\n' '{"marketplaces":[]}' ;;
  'plugin list --json')
    if [ "${MOCK_INSTALLED:-}" = 1 ]; then
      printf '%s\n' '{"installed":[{"pluginId":"development-system@ai-plugins","name":"development-system","version":"6.8.0"}],"available":[]}'
    else
      printf '%s\n' '{"installed":[],"available":[{"name":"github","pluginId":"github@curated"},{"name":"coderabbit","pluginId":"coderabbit@curated"}]}'
    fi
    ;;
  '-C /tmp mcp list --json') printf '[{"name":"development-discipline","transport":{"env":{"PLUGIN_ROOT":"/tmp/cache/development-system/6.8.0","PLUGIN_DATA":"%s"}}},{"name":"tiber","transport":{"env":{"PLUGIN_ROOT":"/tmp/cache/development-system/6.8.0","PLUGIN_DATA":"%s"}}}]\n' "$MOCK_PLUGIN_DATA" "$MOCK_PLUGIN_DATA" ;;
  *) printf '%s\n' "$*" >>"$CODEX_CALL_LOG" ;;
esac
SH
  chmod +x "$TMPROOT/bin/codex"
  export CODEX_CALL_LOG="$TMPROOT/calls"
}
teardown() { rm -rf "$TMPROOT"; }

@test "dry run tracks main and prints optional selections without mutation" {
  run env PATH="$TMPROOT/bin:$PATH" sh "$ROOT/install.sh" --dry-run --with github,coderabbit
  [ "$status" -eq 0 ]
  [[ "$output" == *'codex plugin marketplace add jwilger/ai-plugins --ref main'* ]]
  [[ "$output" == *'codex plugin add development-system@ai-plugins'* ]]
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

@test "installer saves the host signing socket in Codex plugin data" {
  export MOCK_INSTALLED=1
  export MOCK_PLUGIN_DATA="$TMPROOT/plugin-data"
  mkdir -p "$TMPROOT/plugin-data"
  python3 -c 'import socket,sys; s=socket.socket(socket.AF_UNIX); s.bind(sys.argv[1])' "$TMPROOT/agent.sock"
  run env PATH="$TMPROOT/bin:$PATH" SSH_AUTH_SOCK="$TMPROOT/agent.sock" \
    MOCK_INSTALLED=1 MOCK_PLUGIN_DATA="$MOCK_PLUGIN_DATA" \
    sh "$ROOT/install.sh" --with none
  [ "$status" -eq 0 ]
  [ "$(cat "$MOCK_PLUGIN_DATA/signing-agent-socket")" = "$TMPROOT/agent.sock" ]
  [ "$(stat -c %a "$MOCK_PLUGIN_DATA/signing-agent-socket")" = 600 ]
}
