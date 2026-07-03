#!/usr/bin/env bats
# Tests for the cross-harness marketplace manifest sync-validator.

setup() {
  SCRIPT="$BATS_TEST_DIRNAME/../validate-manifests.sh"
  ROOT="$(mktemp -d)"
}

teardown() {
  rm -rf "$ROOT"
}

make_plugin() {
  # make_plugin <name> [claude-name] [codex-name] [claude-version] [codex-version]
  local name="$1" cc="${2:-$1}" cx="${3:-$1}" cc_version="${4:-1.2.3}"
  local cx_version="${5:-$cc_version}"
  mkdir -p "$ROOT/plugins/$name/.claude-plugin" "$ROOT/plugins/$name/.codex-plugin"
  echo "{\"name\":\"$cc\",\"version\":\"$cc_version\"}" >"$ROOT/plugins/$name/.claude-plugin/plugin.json"
  echo "{\"name\":\"$cx\",\"version\":\"$cx_version\"}" >"$ROOT/plugins/$name/.codex-plugin/plugin.json"
}

write_manifests() {
  # write_manifests "<claude names>" "<codex names>"
  mkdir -p "$ROOT/.claude-plugin" "$ROOT/.agents/plugins"
  manifest_for "$1" >"$ROOT/.claude-plugin/marketplace.json"
  manifest_for "$2" >"$ROOT/.agents/plugins/marketplace.json"
}

manifest_for() {
  local entries=""
  for n in $1; do
    entries="$entries{\"name\":\"$n\",\"source\":\"$n\",\"version\":\"1.2.3\"},"
  done
  echo "{\"plugins\":[${entries%,}]}"
}

@test "passes on the real repository" {
  run bash "$SCRIPT"
  [ "$status" -eq 0 ]
}

@test "passes a well-formed fixture" {
  make_plugin alpha
  make_plugin beta
  write_manifests "alpha beta" "alpha beta"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -eq 0 ]
}

@test "fails when the plugin sets differ" {
  make_plugin alpha
  make_plugin beta
  write_manifests "alpha beta" "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"plugin-sets-differ"* ]]
}

@test "fails when a plugin directory is unregistered" {
  make_plugin alpha
  make_plugin beta
  write_manifests "alpha" "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unregistered-plugin"* ]]
}

@test "fails when a manifest lists a plugin with no directory" {
  make_plugin alpha
  write_manifests "alpha ghost" "alpha ghost"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"manifest-plugin-without-dir"* ]]
}

@test "fails when a codex plugin.json is missing" {
  make_plugin alpha
  rm "$ROOT/plugins/alpha/.codex-plugin/plugin.json"
  write_manifests "alpha" "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-codex-plugin-json"* ]]
}

@test "fails when a plugin.json name mismatches its directory" {
  make_plugin alpha alpha wrong-name
  write_manifests "alpha" "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"codex-plugin-name-mismatch"* ]]
}

@test "fails when plugin versions are not semver" {
  make_plugin alpha alpha alpha not-semver not-semver
  write_manifests "alpha" "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-claude-plugin-version"* ]]
}

@test "fails when claude and codex plugin versions differ" {
  make_plugin alpha alpha alpha 1.2.3 1.2.4
  write_manifests "alpha" "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"plugin-version-mismatch"* ]]
}

@test "fails when claude marketplace version differs from plugin version" {
  make_plugin alpha alpha alpha 1.2.4
  write_manifests "alpha" "alpha"
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -ne 0 ]
  [[ "$output" == *"claude-marketplace-version-mismatch"* ]]
}
