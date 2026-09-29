#!/usr/bin/env bats
setup() {
  SOURCE_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  ROOT="$(mktemp -d)"
  mkdir -p "$ROOT/scripts" "$ROOT/plugins/alpha" "$ROOT/.agents/plugins"
  cp "$SOURCE_ROOT/scripts/bump-plugin-version.sh" "$SOURCE_ROOT/scripts/check-plugin-versions.sh" "$ROOT/scripts/"
  printf '{"name":"alpha","version":"1.2.3"}\n' >"$ROOT/plugins/alpha/plugin.json"
  printf '{"plugins":[{"name":"alpha","version":"1.2.3","source":{"source":"local","path":"./plugins/alpha"}}]}\n' >"$ROOT/.agents/plugins/marketplace.json"
  printf '| Plugin | Harness | Description | Version |\n| --- | --- | --- | --- |\n| [alpha](plugins/alpha/README.md) | Codex | Fixture | 1.2.3 |\n' >"$ROOT/README.md"
  git -C "$ROOT" init -q
  git -C "$ROOT" add .
  git -C "$ROOT" -c user.name=Test -c user.email=test@example.com commit -qm baseline
}
teardown() { rm -rf "$ROOT"; }

@test "version bump synchronizes plugin and marketplace metadata" {
  run bash "$ROOT/scripts/bump-plugin-version.sh" alpha minor
  [ "$status" -eq 0 ]
  [ "$(jq -r .version "$ROOT/plugins/alpha/plugin.json")" = 1.3.0 ]
  [ "$(jq -r .plugins[0].version "$ROOT/.agents/plugins/marketplace.json")" = 1.3.0 ]
  grep -Fq '| 1.3.0 |' "$ROOT/README.md"
  run bash "$ROOT/scripts/check-plugin-versions.sh" HEAD
  [ "$status" -eq 0 ]
}

@test "changed plugin with unchanged version is rejected" {
  printf 'new behavior\n' >"$ROOT/plugins/alpha/README.md"
  git -C "$ROOT" add .
  run bash "$ROOT/scripts/check-plugin-versions.sh" HEAD
  [ "$status" -ne 0 ]
  [[ "$output" == *'without a version bump'* ]]
}

@test "changed binary installer with unchanged version is rejected" {
  printf 'changed installer\n' >"$ROOT/scripts/install-alpha-binaries.sh"
  git -C "$ROOT" add .
  run bash "$ROOT/scripts/check-plugin-versions.sh" HEAD
  [ "$status" -ne 0 ]
  [[ "$output" == *'without a version bump'* ]]
}
