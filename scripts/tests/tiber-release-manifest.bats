#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  SCRIPT="$ROOT/scripts/check-tiber-release-manifest.sh"
  COMPLETE_SCRIPT="$ROOT/scripts/check-tiber-release-complete.sh"
  BUILD_ALL_SCRIPT="$ROOT/scripts/build-tiber-release-all.sh"
}

copy_detect_target_helper() {
  mkdir -p "$1/plugins/tiber/scripts"
  cp "$ROOT/plugins/tiber/scripts/detect-target.sh" "$1/plugins/tiber/scripts/detect-target.sh"
}

copy_launcher_helper() {
  mkdir -p "$1/plugins/tiber/bin"
  cp "$ROOT/plugins/tiber/bin/tiber" "$1/plugins/tiber/bin/tiber"
}

write_release_checksums() {
  local fixture="$1"
  : >"$fixture/plugins/tiber/release-binaries.sha256"
  while IFS= read -r binary_path; do
    if [ -e "$fixture/plugins/tiber/$binary_path" ]; then
      sha256sum "$fixture/plugins/tiber/$binary_path" |
        awk -v path="$binary_path" '{ print $1 "  " path }' >>"$fixture/plugins/tiber/release-binaries.sha256"
    else
      printf '0000000000000000000000000000000000000000000000000000000000000000  %s\n' \
        "$binary_path" >>"$fixture/plugins/tiber/release-binaries.sha256"
    fi
  done < <(jq -r '.binaries[].path' "$fixture/plugins/tiber/release-binaries.json")
}

host_release_path() {
  bash -c '
    source "$1"
    host_target="$(detect_tiber_target)"
    jq -r --arg target "$host_target" ".binaries[] | select(.target == \$target) | .path" "$0"
  ' "$ROOT/plugins/tiber/release-binaries.json" "$ROOT/plugins/tiber/scripts/detect-target.sh"
}

@test "real release manifest has an executable host binary" {
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -eq 0 ]
}

@test "host release binary supports agent-unresolvable blocked reason updates" {
  fixture="$(mktemp -d)"
  host_path="$(host_release_path)"
  tiber_bin="$ROOT/plugins/tiber/$host_path"
  git -C "$fixture" init
  git -C "$fixture" config user.name "Tiber Test"
  git -C "$fixture" config user.email "tiber@example.test"
  git -C "$fixture" config commit.gpgsign false
  printf '# Fixture\n' >"$fixture/README.md"
  git -C "$fixture" add README.md
  git -C "$fixture" commit -m "init"

  run bash -c 'cd "$1" && "$2" init' _ "$fixture" "$tiber_bin"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" create "Release binary blocked task"' _ "$fixture" "$tiber_bin"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" update release-binary-blocked-task --agent-blocked-reason "Waiting on external account access."' _ "$fixture" "$tiber_bin"
  [ "$status" -eq 0 ]
  run bash -c 'cd "$1" && "$2" show release-binary-blocked-task' _ "$fixture" "$tiber_bin"

  rm -rf "$fixture"
  if [ "$status" -ne 0 ]; then
    printf '%s\n' "$output"
  fi
  [ "$status" -eq 0 ]
  [[ "$output" == *"agent_blocked_reason: Waiting on external account access."* ]]
}

@test "release manifest check fails when the host binary is missing" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"

  run bash "$SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-host-release-binary"* ]]
}

@test "release manifest check fails when the host binary is empty" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  host_path="$(bash -c '
    source "$1"
    host_target="$(detect_tiber_target)"
    jq -r --arg target "$host_target" ".binaries[] | select(.target == \$target) | .path" "$0"
  ' "$ROOT/plugins/tiber/release-binaries.json" "$ROOT/plugins/tiber/scripts/detect-target.sh")"
  mkdir -p "$fixture/plugins/tiber/$(dirname "$host_path")"
  touch "$fixture/plugins/tiber/$host_path"
  chmod +x "$fixture/plugins/tiber/$host_path"

  run bash "$SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-host-release-binary"* ]]
}

@test "complete release check passes when all target binaries are executable" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  host_path="$(host_release_path)"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/tiber/$(dirname "$binary_path")"
    if [ "$binary_path" = "$host_path" ]; then
      cp "$ROOT/plugins/tiber/$binary_path" "$fixture/plugins/tiber/$binary_path"
    else
      printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/tiber/$binary_path"
      chmod +x "$fixture/plugins/tiber/$binary_path"
    fi
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/tiber/release-binaries.json")
  write_release_checksums "$fixture"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  if [ "$status" -ne 0 ]; then
    printf '%s\n' "$output"
  fi
  [ "$status" -eq 0 ]
}

@test "complete release check fails when any target binary is missing" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  while IFS= read -r binary_path; do
    if [ "$binary_path" = "dist/aarch64-apple-darwin/tiber" ]; then
      continue
    fi
    mkdir -p "$fixture/plugins/tiber/$(dirname "$binary_path")"
    printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/tiber/$binary_path"
    chmod +x "$fixture/plugins/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/tiber/release-binaries.json")
  write_release_checksums "$fixture"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-release-binary target=aarch64-apple-darwin"* ]]
}

@test "complete release check fails when any target binary is empty" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/tiber/$(dirname "$binary_path")"
    if [ "$binary_path" != "dist/aarch64-apple-darwin/tiber" ]; then
      printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/tiber/$binary_path"
    else
      touch "$fixture/plugins/tiber/$binary_path"
    fi
    chmod +x "$fixture/plugins/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/tiber/release-binaries.json")
  write_release_checksums "$fixture"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-release-binary target=aarch64-apple-darwin"* ]]
}

@test "complete release check reports unsupported host target" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber/scripts"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  cat >"$fixture/plugins/tiber/scripts/detect-target.sh" <<'SH'
detect_tiber_target() {
  return 1
}
SH
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/tiber/$(dirname "$binary_path")"
    printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/tiber/$binary_path"
    chmod +x "$fixture/plugins/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/tiber/release-binaries.json")
  write_release_checksums "$fixture"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-host-release-binary"* ]]
}

@test "complete release check fails when host manifest path differs from launcher path" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  host_target="$(bash -c 'source "$1"; detect_tiber_target' _ "$ROOT/plugins/tiber/scripts/detect-target.sh")"
  jq --arg target "$host_target" \
    '(.binaries[] | select(.target == $target) | .path) = "dist/stale-host/tiber"' \
    "$fixture/plugins/tiber/release-binaries.json" >"$fixture/plugins/tiber/release-binaries.json.tmp"
  mv "$fixture/plugins/tiber/release-binaries.json.tmp" "$fixture/plugins/tiber/release-binaries.json"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/tiber/$(dirname "$binary_path")"
    cp "$ROOT/plugins/tiber/dist/$host_target/tiber" "$fixture/plugins/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$fixture/plugins/tiber/release-binaries.json")
  mkdir -p "$fixture/plugins/tiber/dist/$host_target"
  cp "$ROOT/plugins/tiber/dist/$host_target/tiber" "$fixture/plugins/tiber/dist/$host_target/tiber"
  write_release_checksums "$fixture"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-release-manifest-shape"* ]]
}

@test "complete release check fails when launcher is missing" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/tiber/$(dirname "$binary_path")"
    printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/tiber/$binary_path"
    chmod +x "$fixture/plugins/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/tiber/release-binaries.json")

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-release-launcher"* ]]
}

@test "complete release check fails when checksums are missing" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/tiber/$(dirname "$binary_path")"
    cp "$ROOT/plugins/tiber/$binary_path" "$fixture/plugins/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/tiber/release-binaries.json")

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-release-checksums"* ]]
}

@test "complete release check fails when a binary does not match checksum provenance" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/tiber/$(dirname "$binary_path")"
    cp "$ROOT/plugins/tiber/$binary_path" "$fixture/plugins/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/tiber/release-binaries.json")
  write_release_checksums "$fixture"
  printf '\n# stale binary\n' >>"$fixture/plugins/tiber/dist/aarch64-apple-darwin/tiber"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"stale-release-binary target=aarch64-apple-darwin"* ]]
}

@test "complete release check fails when checksum sidecar has stale entries" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/tiber/$(dirname "$binary_path")"
    cp "$ROOT/plugins/tiber/$binary_path" "$fixture/plugins/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/tiber/release-binaries.json")
  write_release_checksums "$fixture"
  printf '0000000000000000000000000000000000000000000000000000000000000000  dist/stale/tiber\n' >>"$fixture/plugins/tiber/release-binaries.sha256"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"release-checksum-paths-mismatch"* ]]
}

@test "release build script reuses an already installed local toolchain" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/scripts"
  cp "$BUILD_ALL_SCRIPT" "$fixture/scripts/build-tiber-release-all.sh"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  cargo_home="$fixture/cargo"
  rustup_home="$fixture/rustup"
  target_dir="$fixture/target"
  mkdir -p "$cargo_home/bin" "$rustup_home/toolchains/stable-x86_64-unknown-linux-gnu/bin"

  cat >"$cargo_home/bin/rustup" <<'EOF'
#!/usr/bin/env bash
set -euo pipefail
if [ "$1" = "toolchain" ] && [ "$2" = "list" ]; then
  echo "stable-x86_64-unknown-linux-gnu (active, default)"
  exit 0
fi
if [ "$1" = "toolchain" ] && [ "$2" = "install" ]; then
  echo "unexpected toolchain install" >&2
  exit 42
fi
if [ "$1" = "target" ] && [ "$2" = "list" ]; then
  printf '%s\n' \
    aarch64-unknown-linux-gnu \
    x86_64-unknown-linux-gnu \
    x86_64-apple-darwin \
    aarch64-apple-darwin
  exit 0
fi
if [ "$1" = "run" ]; then
  shift 2
  if [ "$1" = "rustc" ]; then
    echo "$RUSTUP_HOME/toolchains/stable-x86_64-unknown-linux-gnu"
    exit 0
  fi
fi
echo "unexpected rustup $*" >&2
exit 43
EOF

  cat >"$cargo_home/bin/cargo" <<EOF
#!/usr/bin/env bash
set -euo pipefail
target_dir="$target_dir"
if [ "\$1" = "zigbuild" ]; then
  if [ "\${2:-}" = "--help" ]; then
    exit 0
  fi
  target=""
  while [ "\$#" -gt 0 ]; do
    if [ "\$1" = "--target" ]; then
      target="\$2"
      break
    fi
    shift
  done
  mkdir -p "\$target_dir/\$target/release"
  touch "\$target_dir/\$target/release/tiber"
  chmod +x "\$target_dir/\$target/release/tiber"
  exit 0
fi
if [ "\$1" = "metadata" ]; then
  printf '{"target_directory":"%s"}\n' "\$target_dir"
  exit 0
fi
echo "unexpected cargo \$*" >&2
exit 44
EOF
  chmod +x "$cargo_home/bin/rustup" "$cargo_home/bin/cargo"
  cat >"$cargo_home/bin/zig" <<'EOF'
#!/usr/bin/env bash
exit 0
EOF
  chmod +x "$cargo_home/bin/zig"

  run env RUSTUP_HOME="$rustup_home" CARGO_HOME="$cargo_home" bash "$fixture/scripts/build-tiber-release-all.sh"

  for target in x86_64-unknown-linux-gnu aarch64-unknown-linux-gnu x86_64-apple-darwin aarch64-apple-darwin; do
    [ -x "$fixture/plugins/tiber/dist/$target/tiber" ]
  done
  [ -s "$fixture/plugins/tiber/release-binaries.sha256" ]

  rm -rf "$fixture"
  [ "$status" -eq 0 ]
}
