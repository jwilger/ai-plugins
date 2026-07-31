#!/usr/bin/env bats

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  export REPO_ROOT
  TEST_ROOT="$(mktemp -d)"
  export TEST_ROOT
  mkdir -p "$TEST_ROOT/releases" "$TEST_ROOT/markers" "$TEST_ROOT/home" "$TEST_ROOT/system-bin"
  printf '#!/bin/sh\nexit 127\n' >"$TEST_ROOT/system-bin/bd"
  chmod +x "$TEST_ROOT/system-bin/bd"
  NODE="$(command -v node)"
  export NODE
}

teardown() {
  rm -rf -- "$TEST_ROOT"
}

make_bd_release() {
  local version=$1 archive_path=$2 mode=${3:-normal}
  local release_root="$TEST_ROOT/release-bd-$mode"
  rm -rf "$release_root"
  mkdir -p "$release_root"
  cat >"$release_root/bd" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
if [[ -n "${DEVELOPMENT_SYSTEM_TOOL_TEST_MARKERS:-}" ]]; then
  printf '%s\n' "$0 $*" >>"$DEVELOPMENT_SYSTEM_TOOL_TEST_MARKERS/bd"
fi
case "${1:-}" in
  version|--version)
    if [[ "__MODE__" == fail-after-replace && "$0" == */.local/bin/bd ]]; then
      exit 86
    fi
    printf 'bd version %s\n' "__TOOL_VERSION__"
    ;;
  init)
    mkdir -p .beads
    printf '{}\n' >.beads/metadata.json
    printf '# generated\n' >.beads/README.md
    printf 'backend: dolt\n' >.beads/config.yaml
    printf '.dolt\n' >.beads/.gitignore
    ;;
  where)
    test -d .beads
    printf '%s/.beads\n' "$PWD"
    ;;
  config)
    ;;
  *)
    printf 'bd %s\n' "$*"
    ;;
esac
SH
  sed -e "s/__TOOL_VERSION__/$version/g" -e "s/__MODE__/$mode/g" \
    "$release_root/bd" >"$release_root/bd.expanded"
  mv "$release_root/bd.expanded" "$release_root/bd"
  chmod +x "$release_root/bd"
  tar -C "$release_root" -czf "$archive_path" bd
}

write_manifest() {
  local version=${1:-1.1.2} archive=${2:-$TEST_ROOT/releases/bd.tar.gz}
  make_bd_release "$version" "$archive"
  local hash
  hash="$(sha256sum "$archive" | awk '{print $1}')"
  cat >"$TEST_ROOT/releases.json" <<JSON
{
  "schemaVersion": 2,
  "tools": {
    "bd": {
      "version": "$version",
      "requiredFor": ["beads"],
      "versionCommand": ["version"],
      "versionPattern": "\\\\bbd version (\\\\d+\\\\.\\\\d+\\\\.\\\\d+)\\\\b",
      "releases": {
        "x86_64-linux": {
          "url": "file://$archive",
          "sha256": "$hash",
          "binaryPath": "bd"
        }
      }
    }
  }
}
JSON
}

install_environment() {
  env \
    HOME="$TEST_ROOT/home" \
    PATH="$TEST_ROOT/system-bin:$PATH" \
    DEVELOPMENT_SYSTEM_TOOL_RELEASES="$TEST_ROOT/releases.json" \
    DEVELOPMENT_SYSTEM_TOOL_ALLOW_FILE_URLS=1 \
    DEVELOPMENT_SYSTEM_TOOL_PLATFORM=linux \
    DEVELOPMENT_SYSTEM_TOOL_ARCH=x64 \
    DEVELOPMENT_SYSTEM_TOOL_TEST_MARKERS="$TEST_ROOT/markers" \
    "$@"
}

initialize_project_with_enabled_beads() {
  git -C "$TEST_ROOT" init -q --initial-branch=main project
  git -C "$TEST_ROOT/project" config user.email test@example.com
  git -C "$TEST_ROOT/project" config user.name "Test User"
  touch "$TEST_ROOT/project/README.md"
  git -C "$TEST_ROOT/project" add README.md
  git -C "$TEST_ROOT/project" commit -qm "test: initialize fixture"
  cat >"$TEST_ROOT/project/.development-system.toml" <<'TOML'
schema_version = 2

[delivery]
mode = "direct-to-trunk"
trunk_branch = "main"

[features]
worktrees = true
beads = true
agentic_systems = false
eval_case_reporting = false

[worktrees]
root = ".worktrees"

[beads]
workflow = "development-change-direct"
TOML
  mkdir -p "$TEST_ROOT/project/.beads/formulas"
  printf '{}\n' >"$TEST_ROOT/project/.beads/metadata.json"
  printf '# generated\n' >"$TEST_ROOT/project/.beads/README.md"
  printf 'backend: dolt\n' >"$TEST_ROOT/project/.beads/config.yaml"
  printf '.dolt\n' >"$TEST_ROOT/project/.beads/.gitignore"
  cp "$REPO_ROOT/plugins/development-system/formulas/"*.formula.toml \
    "$TEST_ROOT/project/.beads/formulas/"
  git -C "$TEST_ROOT/project" add .development-system.toml .beads
  git -C "$TEST_ROOT/project" commit -qm "test: configure development system"
}

@test "verified pinned bd is installed user-globally and no standalone Dolt is installed" {
  write_manifest

  run install_environment "$NODE" \
    "$REPO_ROOT/plugins/development-system/bin/install-development-tool.mjs" \
    install --json

  [ "$status" -eq 0 ]
  printf '%s\n' "$output" | jq -e '
    .installationScope == "user-global" and
    .requiresSudo == false and
    .installed == ["bd"] and
    (.tools | map({name, targetVersion, status})) ==
      [{name:"bd", targetVersion:"1.1.2", status:"compatible"}]
  ' >/dev/null
  [ -x "$TEST_ROOT/home/.local/bin/bd" ]
  [ ! -e "$TEST_ROOT/home/.local/bin/dolt" ]
  [ -z "$(find "$TEST_ROOT/home/.local/bin" -maxdepth 1 -name '.development-system-*' -print -quit)" ]
}

@test "checksum failure preserves an existing working bd" {
  write_manifest
  mkdir -p "$TEST_ROOT/home/.local/bin"
  make_bd_release 1.1.1 "$TEST_ROOT/releases/old.tar.gz"
  tar -xzf "$TEST_ROOT/releases/old.tar.gz" -C "$TEST_ROOT/home/.local/bin"
  local before
  before="$(sha256sum "$TEST_ROOT/home/.local/bin/bd")"
  jq '.tools.bd.releases["x86_64-linux"].sha256 = ("0" * 64)' \
    "$TEST_ROOT/releases.json" >"$TEST_ROOT/bad-releases.json"
  mv "$TEST_ROOT/bad-releases.json" "$TEST_ROOT/releases.json"

  run install_environment "$NODE" \
    "$REPO_ROOT/plugins/development-system/bin/install-development-tool.mjs" install

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.tool_checksum_failed"* ]]
  [ "$(sha256sum "$TEST_ROOT/home/.local/bin/bd")" = "$before" ]
  [ "$(HOME="$TEST_ROOT/home" "$TEST_ROOT/home/.local/bin/bd" version)" = "bd version 1.1.1" ]
}

@test "download failure leaves the user-global destination unchanged" {
  write_manifest
  jq --arg url "file://$TEST_ROOT/releases/missing.tar.gz" \
    '.tools.bd.releases["x86_64-linux"].url = $url' \
    "$TEST_ROOT/releases.json" >"$TEST_ROOT/missing-releases.json"
  mv "$TEST_ROOT/missing-releases.json" "$TEST_ROOT/releases.json"

  run install_environment "$NODE" \
    "$REPO_ROOT/plugins/development-system/bin/install-development-tool.mjs" install

  [ "$status" -eq 2 ]
  [[ "$output" == *"ENOENT"* || "$output" == *"tool_download_failed"* ]]
  [ ! -e "$TEST_ROOT/home/.local/bin/bd" ]
}

@test "mutable non-HTTPS release URLs are rejected outside the explicit fixture mode" {
  write_manifest

  run env \
    HOME="$TEST_ROOT/home" \
    PATH="$TEST_ROOT/system-bin:$PATH" \
    DEVELOPMENT_SYSTEM_TOOL_RELEASES="$TEST_ROOT/releases.json" \
    DEVELOPMENT_SYSTEM_TOOL_PLATFORM=linux \
    DEVELOPMENT_SYSTEM_TOOL_ARCH=x64 \
    "$NODE" "$REPO_ROOT/plugins/development-system/bin/install-development-tool.mjs" install

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.tool_url_unsupported protocol=file:"* ]]
  [ ! -e "$TEST_ROOT/home/.local/bin/bd" ]
}

@test "malformed release archive fails extraction without replacing bd" {
  printf 'not a tar archive\n' >"$TEST_ROOT/releases/bd.tar.gz"
  local hash
  hash="$(sha256sum "$TEST_ROOT/releases/bd.tar.gz" | awk '{print $1}')"
  write_manifest
  printf 'not a tar archive\n' >"$TEST_ROOT/releases/bd.tar.gz"
  hash="$(sha256sum "$TEST_ROOT/releases/bd.tar.gz" | awk '{print $1}')"
  jq --arg hash "$hash" '.tools.bd.releases["x86_64-linux"].sha256 = $hash' \
    "$TEST_ROOT/releases.json" >"$TEST_ROOT/malformed.json"
  mv "$TEST_ROOT/malformed.json" "$TEST_ROOT/releases.json"

  run install_environment "$NODE" \
    "$REPO_ROOT/plugins/development-system/bin/install-development-tool.mjs" install

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.tool_extract_failed"* ]]
  [ ! -e "$TEST_ROOT/home/.local/bin/bd" ]
}

@test "post-replacement verification failure atomically restores the previous bd" {
  local archive="$TEST_ROOT/releases/bd.tar.gz"
  make_bd_release 1.1.2 "$archive" fail-after-replace
  local hash
  hash="$(sha256sum "$archive" | awk '{print $1}')"
  write_manifest
  make_bd_release 1.1.2 "$archive" fail-after-replace
  hash="$(sha256sum "$archive" | awk '{print $1}')"
  jq --arg hash "$hash" '.tools.bd.releases["x86_64-linux"].sha256 = $hash' \
    "$TEST_ROOT/releases.json" >"$TEST_ROOT/failing.json"
  mv "$TEST_ROOT/failing.json" "$TEST_ROOT/releases.json"
  mkdir -p "$TEST_ROOT/home/.local/bin"
  make_bd_release 1.1.1 "$TEST_ROOT/releases/old.tar.gz"
  tar -xzf "$TEST_ROOT/releases/old.tar.gz" -C "$TEST_ROOT/home/.local/bin"

  run install_environment "$NODE" \
    "$REPO_ROOT/plugins/development-system/bin/install-development-tool.mjs" install

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.tool_installed_version_invalid"* ]]
  [ "$(HOME="$TEST_ROOT/home" "$TEST_ROOT/home/.local/bin/bd" version)" = "bd version 1.1.1" ]
}

@test "archive size and download time are bounded" {
  write_manifest

  run env \
    HOME="$TEST_ROOT/home" \
    PATH="$TEST_ROOT/system-bin:$PATH" \
    DEVELOPMENT_SYSTEM_TOOL_RELEASES="$TEST_ROOT/releases.json" \
    DEVELOPMENT_SYSTEM_TOOL_ALLOW_FILE_URLS=1 \
    DEVELOPMENT_SYSTEM_TOOL_PLATFORM=linux \
    DEVELOPMENT_SYSTEM_TOOL_ARCH=x64 \
    DEVELOPMENT_SYSTEM_TOOL_MAX_ARCHIVE_BYTES=8 \
    "$NODE" "$REPO_ROOT/plugins/development-system/bin/install-development-tool.mjs" install
  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.tool_download_size_exceeded"* ]]

  mkfifo "$TEST_ROOT/releases/stalled"
  jq --arg url "file://$TEST_ROOT/releases/stalled" '
    .tools.bd.releases["x86_64-linux"].url = $url |
    .tools.bd.releases["x86_64-linux"].sha256 = ("0" * 64)
  ' "$TEST_ROOT/releases.json" >"$TEST_ROOT/stalled.json"
  (exec 3>"$TEST_ROOT/releases/stalled"; sleep 5) &
  local writer=$!
  run env \
    HOME="$TEST_ROOT/home" \
    PATH="$TEST_ROOT/system-bin:$PATH" \
    DEVELOPMENT_SYSTEM_TOOL_RELEASES="$TEST_ROOT/stalled.json" \
    DEVELOPMENT_SYSTEM_TOOL_ALLOW_FILE_URLS=1 \
    DEVELOPMENT_SYSTEM_TOOL_PLATFORM=linux \
    DEVELOPMENT_SYSTEM_TOOL_ARCH=x64 \
    DEVELOPMENT_SYSTEM_TOOL_DOWNLOAD_TIMEOUT_MS=50 \
    "$NODE" "$REPO_ROOT/plugins/development-system/bin/install-development-tool.mjs" install
  kill "$writer" 2>/dev/null || true
  wait "$writer" 2>/dev/null || true
  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.tool_download_timeout"* ]]
}

@test "unsupported host targets fail before downloading or writing a tool" {
  write_manifest

  run env \
    HOME="$TEST_ROOT/home" \
    PATH="$TEST_ROOT/system-bin:$PATH" \
    DEVELOPMENT_SYSTEM_TOOL_RELEASES="$TEST_ROOT/releases.json" \
    DEVELOPMENT_SYSTEM_TOOL_ALLOW_FILE_URLS=1 \
    DEVELOPMENT_SYSTEM_TOOL_PLATFORM=freebsd \
    DEVELOPMENT_SYSTEM_TOOL_ARCH=x64 \
    "$NODE" "$REPO_ROOT/plugins/development-system/bin/install-development-tool.mjs" install

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.tool_platform_unsupported"* ]]
  [ ! -e "$TEST_ROOT/home/.local/bin/bd" ]
}

@test "unchanged enabled Beads policy still installs missing bd user-globally without a repository commit" {
  write_manifest
  initialize_project_with_enabled_beads
  printf '#!/bin/sh\nexit 127\n' >"$TEST_ROOT/system-bin/bd"
  chmod +x "$TEST_ROOT/system-bin/bd"
  local before_head
  before_head="$(git -C "$TEST_ROOT/project" rev-parse HEAD)"

  run install_environment env \
    PACKAGE_ROOT="$REPO_ROOT/plugins/development-system" \
    PROJECT="$TEST_ROOT/project" \
    "$NODE" --experimental-strip-types --input-type=module -e '
      const { applySetupPreview, createSetupPreview } = await import(`${process.env.PACKAGE_ROOT}/extensions/development-system/adapters/setup.ts`);
      const preview = await createSetupPreview(process.env.PACKAGE_ROOT, process.env.PROJECT, "--enable beads");
      process.stdout.write(await applySetupPreview(process.env.PACKAGE_ROOT, preview));
    '

  [ "$status" -eq 0 ]
  [[ "$output" == *"development_system.setup_configuration_unchanged"* ]]
  [[ "$output" == *"development_system.setup_tools_installed bd=1.1.2"* ]]
  [ -x "$TEST_ROOT/home/.local/bin/bd" ]
  [ ! -e "$TEST_ROOT/home/.local/bin/dolt" ]
  [ "$(git -C "$TEST_ROOT/project" rev-parse HEAD)" = "$before_head" ]
  [ -z "$(git -C "$TEST_ROOT/project" status --porcelain)" ]
}

@test "compatible ambient bd is used without a user-global installation" {
  initialize_project_with_enabled_beads
  cat >"$TEST_ROOT/system-bin/bd" <<'SH'
#!/bin/sh
case "${1:-}" in
  version) printf 'bd version 1.1.2\n' ;;
  where) test -d .beads ;;
  config) ;;
esac
SH
  chmod +x "$TEST_ROOT/system-bin/bd"

  run env HOME="$TEST_ROOT/home" PATH="$TEST_ROOT/system-bin:$PATH" \
    PACKAGE_ROOT="$REPO_ROOT/plugins/development-system" PROJECT="$TEST_ROOT/project" \
    "$NODE" --experimental-strip-types --input-type=module -e '
      const { applySetupPreview, createSetupPreview } = await import(`${process.env.PACKAGE_ROOT}/extensions/development-system/adapters/setup.ts`);
      const preview = await createSetupPreview(process.env.PACKAGE_ROOT, process.env.PROJECT, "--enable beads");
      process.stdout.write(await applySetupPreview(process.env.PACKAGE_ROOT, preview));
    '

  [ "$status" -eq 0 ]
  [[ "$output" == *"development_system.setup_tools_compatible bd=1.1.2"* ]]
  [ ! -e "$TEST_ROOT/home/.local/bin/bd" ]
}

@test "outdated ambient bd is updated in the user-global destination" {
  write_manifest
  printf '#!/bin/sh\nprintf "bd version 1.1.1\\n"\n' >"$TEST_ROOT/system-bin/bd"
  chmod +x "$TEST_ROOT/system-bin/bd"

  run install_environment "$NODE" \
    "$REPO_ROOT/plugins/development-system/bin/install-development-tool.mjs" install --json

  [ "$status" -eq 0 ]
  printf '%s\n' "$output" | jq -e '.installed == ["bd"]' >/dev/null
  [ "$(HOME="$TEST_ROOT/home" "$TEST_ROOT/home/.local/bin/bd" version)" = "bd version 1.1.2" ]
}

@test "package bd wrapper fails closed with the manifest minimum and setup retry when bd is unavailable" {
  write_manifest

  run env \
    HOME="$TEST_ROOT/home" \
    PATH="$TEST_ROOT/system-bin:$PATH" \
    DEVELOPMENT_SYSTEM_TOOL_RELEASES="$TEST_ROOT/releases.json" \
    DEVELOPMENT_SYSTEM_TOOL_PLATFORM=linux \
    DEVELOPMENT_SYSTEM_TOOL_ARCH=x64 \
    "$REPO_ROOT/plugins/development-system/bin/bd" version

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.tool_unavailable tool=bd minimum=1.1.2"* ]]
  [[ "$output" == *"development-system setup"* ]]
  [ ! -e "$TEST_ROOT/home/.cache/development-system/tools" ]
}
