#!/usr/bin/env bats

setup() {
  REPO_ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  export REPO_ROOT
  TEST_ROOT="$(mktemp -d)"
  export TEST_ROOT
  mkdir -p "$TEST_ROOT/releases" "$TEST_ROOT/tools" "$TEST_ROOT/markers"
}

teardown() {
  rm -rf -- "$TEST_ROOT"
}

make_release() {
  local tool=$1 version=$2 archive_path=$3
  local release_root="$TEST_ROOT/release-$tool"
  mkdir -p "$release_root"
  cat >"$release_root/$tool" <<'SH'
#!/usr/bin/env bash
set -euo pipefail
printf '%s\n' "$0 $*" >>"$DEVELOPMENT_SYSTEM_TOOL_TEST_MARKERS/__TOOL_NAME__"
case "${1:-}" in
  version|--version)
    if [[ "__TOOL_NAME__" == bd ]]; then
      printf 'bd version %s\n' "__TOOL_VERSION__"
    else
      printf 'dolt version %s\n' "__TOOL_VERSION__"
    fi
    ;;
  init)
    mkdir -p .beads
    printf '{}\n' >.beads/metadata.json
    printf '# generated\n' >.beads/README.md
    printf 'backend: dolt\n' >.beads/config.yaml
    printf '.dolt\n' >.beads/.gitignore
    ;;
  where)
    printf '%s/.beads\n' "$PWD"
    ;;
  config)
    ;;
  *)
    printf '%s %s\n' "__TOOL_NAME__" "$*"
    ;;
esac
SH
  chmod +x "$release_root/$tool"
  sed -e "s/__TOOL_NAME__/$tool/g" -e "s/__TOOL_VERSION__/$version/g" \
    "$release_root/$tool" >"$release_root/$tool.expanded"
  mv "$release_root/$tool.expanded" "$release_root/$tool"
  chmod +x "$release_root/$tool"
  tar -C "$release_root" -czf "$archive_path" "$tool"
}

write_manifest() {
  local bd_archive="$TEST_ROOT/releases/bd.tar.gz"
  local dolt_archive="$TEST_ROOT/releases/dolt.tar.gz"
  make_release bd 1.1.2 "$bd_archive"
  make_release dolt 2.2.3 "$dolt_archive"
  local bd_hash dolt_hash
  bd_hash="$(sha256sum "$bd_archive" | awk '{print $1}')"
  dolt_hash="$(sha256sum "$dolt_archive" | awk '{print $1}')"
  cat >"$TEST_ROOT/releases.json" <<JSON
{
  "schemaVersion": 1,
  "tools": {
    "bd": {
      "version": "1.1.2",
      "releases": {
        "x86_64-linux": {
          "url": "file://$bd_archive",
          "sha256": "$bd_hash",
          "binaryPath": "bd"
        }
      }
    },
    "dolt": {
      "version": "2.2.3",
      "releases": {
        "x86_64-linux": {
          "url": "file://$dolt_archive",
          "sha256": "$dolt_hash",
          "binaryPath": "dolt"
        }
      }
    }
  }
}
JSON
}

@test "plugin launchers install verified pinned Beads and Dolt releases on first use" {
  write_manifest

  run env \
    DEVELOPMENT_SYSTEM_TOOL_RELEASES="$TEST_ROOT/releases.json" \
    DEVELOPMENT_SYSTEM_TOOLS_DIR="$TEST_ROOT/tools" \
    DEVELOPMENT_SYSTEM_TOOL_PLATFORM=linux \
    DEVELOPMENT_SYSTEM_TOOL_ARCH=x64 \
    DEVELOPMENT_SYSTEM_TOOL_TEST_MARKERS="$TEST_ROOT/markers" \
    "$REPO_ROOT/plugins/development-system/bin/bd" version
  [ "$status" -eq 0 ]
  [[ "$output" == *"bd version 1.1.2"* ]]

  run env \
    DEVELOPMENT_SYSTEM_TOOL_RELEASES="$TEST_ROOT/releases.json" \
    DEVELOPMENT_SYSTEM_TOOLS_DIR="$TEST_ROOT/tools" \
    DEVELOPMENT_SYSTEM_TOOL_PLATFORM=linux \
    DEVELOPMENT_SYSTEM_TOOL_ARCH=x64 \
    DEVELOPMENT_SYSTEM_TOOL_TEST_MARKERS="$TEST_ROOT/markers" \
    "$REPO_ROOT/plugins/development-system/bin/dolt" version
  [ "$status" -eq 0 ]
  [[ "$output" == *"dolt version 2.2.3"* ]]
  [ -x "$TEST_ROOT/tools/bd/1.1.2/x86_64-linux/bd" ]
  [ -x "$TEST_ROOT/tools/dolt/2.2.3/x86_64-linux/dolt" ]
}

@test "tool installation rejects a release whose checksum does not match" {
  write_manifest
  jq '.tools.bd.releases["x86_64-linux"].sha256 = ("0" * 64)' \
    "$TEST_ROOT/releases.json" >"$TEST_ROOT/bad-releases.json"

  run env \
    DEVELOPMENT_SYSTEM_TOOL_RELEASES="$TEST_ROOT/bad-releases.json" \
    DEVELOPMENT_SYSTEM_TOOLS_DIR="$TEST_ROOT/tools" \
    DEVELOPMENT_SYSTEM_TOOL_PLATFORM=linux \
    DEVELOPMENT_SYSTEM_TOOL_ARCH=x64 \
    "$REPO_ROOT/plugins/development-system/bin/bd" version

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.tool_checksum_failed"* ]]
  [ ! -e "$TEST_ROOT/tools/bd/1.1.2/x86_64-linux/bd" ]
}

@test "tool installation rejects unsupported host targets without downloading" {
  write_manifest

  run env \
    DEVELOPMENT_SYSTEM_TOOL_RELEASES="$TEST_ROOT/releases.json" \
    DEVELOPMENT_SYSTEM_TOOLS_DIR="$TEST_ROOT/tools" \
    DEVELOPMENT_SYSTEM_TOOL_PLATFORM=freebsd \
    DEVELOPMENT_SYSTEM_TOOL_ARCH=x64 \
    "$REPO_ROOT/plugins/development-system/bin/bd" version

  [ "$status" -eq 2 ]
  [[ "$output" == *"development_system.tool_platform_unsupported"* ]]
  [ ! -e "$TEST_ROOT/tools/bd" ]
}

@test "approved setup provisions both plugin-owned tools before initializing Beads" {
  write_manifest
  git -C "$TEST_ROOT" init -q --initial-branch=main project
  git -C "$TEST_ROOT/project" config user.email test@example.com
  git -C "$TEST_ROOT/project" config user.name "Test User"
  touch "$TEST_ROOT/project/README.md"
  git -C "$TEST_ROOT/project" add README.md
  git -C "$TEST_ROOT/project" commit -qm "test: initialize fixture"
  mkdir -p "$TEST_ROOT/system-bin"
  printf '#!/bin/sh\nexit 127\n' >"$TEST_ROOT/system-bin/bd"
  printf '#!/bin/sh\nexit 127\n' >"$TEST_ROOT/system-bin/dolt"
  chmod +x "$TEST_ROOT/system-bin/bd" "$TEST_ROOT/system-bin/dolt"

  run env \
    PATH="$TEST_ROOT/system-bin:$PATH" \
    DEVELOPMENT_SYSTEM_TOOL_RELEASES="$TEST_ROOT/releases.json" \
    DEVELOPMENT_SYSTEM_TOOLS_DIR="$TEST_ROOT/tools" \
    DEVELOPMENT_SYSTEM_TOOL_PLATFORM=linux \
    DEVELOPMENT_SYSTEM_TOOL_ARCH=x64 \
    DEVELOPMENT_SYSTEM_TOOL_TEST_MARKERS="$TEST_ROOT/markers" \
    "$REPO_ROOT/plugins/development-system/bin/development-system" \
    setup --project "$TEST_ROOT/project" --apply --yes

  [ "$status" -eq 0 ]
  [[ "$output" == *"development_system.setup_applied"* ]]
  grep -Fq -- "version" "$TEST_ROOT/markers/dolt"
  grep -Fq -- "version" "$TEST_ROOT/markers/bd"
  grep -Fq -- "init" "$TEST_ROOT/markers/bd"
  [ -f "$TEST_ROOT/project/.beads/formulas/behavior-slice.formula.toml" ]
  [ "$(git -C "$TEST_ROOT/project" rev-list --count HEAD)" -eq 2 ]
}

@test "existing-policy setup uses the same provisioning fallback" {
  write_manifest
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
beads = false
agentic_systems = false
eval_case_reporting = false

[worktrees]
root = ".worktrees"

[beads]
workflow = "development-change-direct"
TOML
  git -C "$TEST_ROOT/project" add .development-system.toml
  git -C "$TEST_ROOT/project" commit -qm "test: configure development system"
  mkdir -p "$TEST_ROOT/system-bin"
  printf '#!/bin/sh\nexit 127\n' >"$TEST_ROOT/system-bin/bd"
  printf '#!/bin/sh\nexit 127\n' >"$TEST_ROOT/system-bin/dolt"
  chmod +x "$TEST_ROOT/system-bin/bd" "$TEST_ROOT/system-bin/dolt"

  run env \
    PATH="$TEST_ROOT/system-bin:$PATH" \
    DEVELOPMENT_SYSTEM_TOOL_RELEASES="$TEST_ROOT/releases.json" \
    DEVELOPMENT_SYSTEM_TOOLS_DIR="$TEST_ROOT/tools" \
    DEVELOPMENT_SYSTEM_TOOL_PLATFORM=linux \
    DEVELOPMENT_SYSTEM_TOOL_ARCH=x64 \
    DEVELOPMENT_SYSTEM_TOOL_TEST_MARKERS="$TEST_ROOT/markers" \
    PACKAGE_ROOT="$REPO_ROOT/plugins/development-system" \
    PROJECT="$TEST_ROOT/project" \
    node --experimental-strip-types --input-type=module -e '
      const { applySetupPreview, createSetupPreview } = await import(`${process.env.PACKAGE_ROOT}/extensions/development-system/adapters/setup.ts`);
      const preview = await createSetupPreview(process.env.PACKAGE_ROOT, process.env.PROJECT, "--enable beads");
      process.stdout.write(await applySetupPreview(process.env.PACKAGE_ROOT, preview));
    '

  [ "$status" -eq 0 ]
  [[ "$output" == *"development_system.setup_applied"* ]]
  grep -Fq -- "version" "$TEST_ROOT/markers/dolt"
  grep -Fq -- "init" "$TEST_ROOT/markers/bd"
  grep -Fq 'beads = true' "$TEST_ROOT/project/.development-system.toml"
  [ "$(git -C "$TEST_ROOT/project" rev-list --count HEAD)" -eq 3 ]
}
