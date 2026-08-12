#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  SCRIPT="$ROOT/scripts/check-tiber-release-manifest.sh"
  COMPLETE_SCRIPT="$ROOT/scripts/check-tiber-release-complete.sh"
  BUILD_ALL_SCRIPT="$ROOT/scripts/build-tiber-release-all.sh"
}

copy_detect_target_helper() {
  mkdir -p "$1/plugins/development-system/components/tiber/scripts"
  cp "$ROOT/plugins/development-system/components/tiber/scripts/detect-target.sh" "$1/plugins/development-system/components/tiber/scripts/detect-target.sh"
}

copy_launcher_helper() {
  mkdir -p "$1/plugins/development-system/components/tiber/bin"
  cp "$ROOT/plugins/development-system/components/tiber/bin/tiber" "$1/plugins/development-system/components/tiber/bin/tiber"
}

write_release_checksums() {
  local fixture="$1"
  local fingerprint
  fingerprint="$(jq -r '.source_fingerprint' "$fixture/plugins/development-system/components/tiber/release-binaries.json")"
  : >"$fixture/plugins/development-system/components/tiber/release-binaries.sha256"
  while IFS= read -r binary_path; do
    if [ -e "$fixture/plugins/development-system/components/tiber/$binary_path" ]; then
      if [ -s "$fixture/plugins/development-system/components/tiber/$binary_path" ] \
        && ! grep -aFq "$fingerprint" "$fixture/plugins/development-system/components/tiber/$binary_path"; then
        printf '\n%s\n' "$fingerprint" >>"$fixture/plugins/development-system/components/tiber/$binary_path"
      fi
      sha256sum "$fixture/plugins/development-system/components/tiber/$binary_path" |
        awk -v path="$binary_path" '{ print $1 "  " path }' >>"$fixture/plugins/development-system/components/tiber/release-binaries.sha256"
    else
      printf '0000000000000000000000000000000000000000000000000000000000000000  %s\n' \
        "$binary_path" >>"$fixture/plugins/development-system/components/tiber/release-binaries.sha256"
    fi
  done < <(jq -r '.binaries[].path' "$fixture/plugins/development-system/components/tiber/release-binaries.json")
}

host_release_path() {
  bash -c '
    source "$1"
    host_target="$(detect_tiber_target)"
    jq -r --arg target "$host_target" ".binaries[] | select(.target == \$target) | .path" "$0"
  ' "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$ROOT/plugins/development-system/components/tiber/scripts/detect-target.sh"
}

@test "real release manifest has an executable host binary" {
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -eq 0 ]
}

@test "source fingerprint changes with shipping Rust source" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber"
  cp -R "$ROOT/plugins/development-system/components/tiber/rust" \
    "$fixture/plugins/development-system/components/tiber/rust"

  before="$(bash "$ROOT/scripts/tiber-source-fingerprint.sh" "$fixture")"
  printf '\n// fingerprint mutation fixture\n' >> \
    "$fixture/plugins/development-system/components/tiber/rust/crates/tiber-core/src/lib.rs"
  after="$(bash "$ROOT/scripts/tiber-source-fingerprint.sh" "$fixture")"

  rm -rf "$fixture"
  [ "$before" != "$after" ]
}

@test "source fingerprint ignores checkout location and cache noise" {
  first="$(mktemp -d)"
  second="$(mktemp -d)"
  for fixture in "$first" "$second"; do
    mkdir -p "$fixture/plugins/development-system/components/tiber"
    cp -R "$ROOT/plugins/development-system/components/tiber/rust" \
      "$fixture/plugins/development-system/components/tiber/rust"
  done
  mkdir -p "$second/plugins/development-system/components/tiber/rust/.dependencies/cache"
  printf 'ignored cache noise\n' > \
    "$second/plugins/development-system/components/tiber/rust/.dependencies/cache/noise.rs"

  first_fingerprint="$(bash "$ROOT/scripts/tiber-source-fingerprint.sh" "$first")"
  second_fingerprint="$(bash "$ROOT/scripts/tiber-source-fingerprint.sh" "$second")"

  rm -rf "$first" "$second"
  [ "$first_fingerprint" = "$second_fingerprint" ]
}

@test "release manifest check fails when the host binary is missing" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"

  run bash "$SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-host-release-binary"* ]]
}

@test "release manifest check fails when the host binary is empty" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  host_path="$(bash -c '
    source "$1"
    host_target="$(detect_tiber_target)"
    jq -r --arg target "$host_target" ".binaries[] | select(.target == \$target) | .path" "$0"
  ' "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$ROOT/plugins/development-system/components/tiber/scripts/detect-target.sh")"
  mkdir -p "$fixture/plugins/development-system/components/tiber/$(dirname "$host_path")"
  touch "$fixture/plugins/development-system/components/tiber/$host_path"
  chmod +x "$fixture/plugins/development-system/components/tiber/$host_path"

  run bash "$SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-host-release-binary"* ]]
}

@test "complete release check passes when all target binaries are executable" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  host_path="$(host_release_path)"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/development-system/components/tiber/$(dirname "$binary_path")"
    if [ "$binary_path" = "$host_path" ]; then
      cp "$ROOT/plugins/development-system/components/tiber/$binary_path" "$fixture/plugins/development-system/components/tiber/$binary_path"
    else
      printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/development-system/components/tiber/$binary_path"
      chmod +x "$fixture/plugins/development-system/components/tiber/$binary_path"
    fi
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/development-system/components/tiber/release-binaries.json")
  write_release_checksums "$fixture"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  if [ "$status" -ne 0 ]; then
    printf '%s\n' "$output"
  fi
  [ "$status" -eq 0 ]
}

@test "complete release check fails when the supported binary is missing" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  while IFS= read -r binary_path; do
    if [ "$binary_path" = "dist/x86_64-unknown-linux-gnu/tiber" ]; then
      continue
    fi
    mkdir -p "$fixture/plugins/development-system/components/tiber/$(dirname "$binary_path")"
    printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/development-system/components/tiber/$binary_path"
    chmod +x "$fixture/plugins/development-system/components/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/development-system/components/tiber/release-binaries.json")
  write_release_checksums "$fixture"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-host-release-binary target=x86_64-unknown-linux-gnu"* ]]
}

@test "complete release check fails when the supported binary is empty" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/development-system/components/tiber/$(dirname "$binary_path")"
    if [ "$binary_path" != "dist/x86_64-unknown-linux-gnu/tiber" ]; then
      printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/development-system/components/tiber/$binary_path"
    else
      touch "$fixture/plugins/development-system/components/tiber/$binary_path"
    fi
    chmod +x "$fixture/plugins/development-system/components/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/development-system/components/tiber/release-binaries.json")
  write_release_checksums "$fixture"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-host-release-binary target=x86_64-unknown-linux-gnu"* ]]
}

@test "complete release check reports unsupported host target" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber/scripts"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  cat >"$fixture/plugins/development-system/components/tiber/scripts/detect-target.sh" <<'SH'
detect_tiber_target() {
  return 1
}
SH
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/development-system/components/tiber/$(dirname "$binary_path")"
    printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/development-system/components/tiber/$binary_path"
    chmod +x "$fixture/plugins/development-system/components/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/development-system/components/tiber/release-binaries.json")
  write_release_checksums "$fixture"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"unsupported-host-release-binary"* ]]
}

@test "complete release check fails when host manifest path differs from launcher path" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  host_target="$(bash -c 'source "$1"; detect_tiber_target' _ "$ROOT/plugins/development-system/components/tiber/scripts/detect-target.sh")"
  jq --arg target "$host_target" \
    '(.binaries[] | select(.target == $target) | .path) = "dist/stale-host/tiber"' \
    "$fixture/plugins/development-system/components/tiber/release-binaries.json" >"$fixture/plugins/development-system/components/tiber/release-binaries.json.tmp"
  mv "$fixture/plugins/development-system/components/tiber/release-binaries.json.tmp" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/development-system/components/tiber/$(dirname "$binary_path")"
    cp "$ROOT/plugins/development-system/components/tiber/dist/$host_target/tiber" "$fixture/plugins/development-system/components/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$fixture/plugins/development-system/components/tiber/release-binaries.json")
  mkdir -p "$fixture/plugins/development-system/components/tiber/dist/$host_target"
  cp "$ROOT/plugins/development-system/components/tiber/dist/$host_target/tiber" "$fixture/plugins/development-system/components/tiber/dist/$host_target/tiber"
  write_release_checksums "$fixture"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-release-manifest-shape"* ]]
}

@test "complete release check fails when launcher is missing" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/development-system/components/tiber/$(dirname "$binary_path")"
    printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/development-system/components/tiber/$binary_path"
    chmod +x "$fixture/plugins/development-system/components/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/development-system/components/tiber/release-binaries.json")

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-release-launcher"* ]]
}

@test "complete release check fails when checksums are missing" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/development-system/components/tiber/$(dirname "$binary_path")"
    cp "$ROOT/plugins/development-system/components/tiber/$binary_path" "$fixture/plugins/development-system/components/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/development-system/components/tiber/release-binaries.json")

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-release-checksums"* ]]
}

@test "complete release check fails when a binary does not match checksum provenance" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/development-system/components/tiber/$(dirname "$binary_path")"
    cp "$ROOT/plugins/development-system/components/tiber/$binary_path" "$fixture/plugins/development-system/components/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/development-system/components/tiber/release-binaries.json")
  write_release_checksums "$fixture"
  printf '\n# stale binary\n' >>"$fixture/plugins/development-system/components/tiber/dist/x86_64-unknown-linux-gnu/tiber"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"stale-release-binary target=x86_64-unknown-linux-gnu"* ]]
}

@test "complete release check rejects a stale binary with refreshed manifest and checksum" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber/dist/x86_64-unknown-linux-gnu"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" \
    "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  stale="$fixture/plugins/development-system/components/tiber/dist/x86_64-unknown-linux-gnu/tiber"
  printf '#!/usr/bin/env sh\nexit 0\n' >"$stale"
  chmod +x "$stale"
  hash="$(sha256sum "$stale" | awk '{ print $1 }')"
  printf '%s  %s\n' "$hash" 'dist/x86_64-unknown-linux-gnu/tiber' > \
    "$fixture/plugins/development-system/components/tiber/release-binaries.sha256"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"release-binary-source-fingerprint-mismatch"* ]]
}

@test "complete release check fails when checksum sidecar has stale entries" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/development-system/components/tiber"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
  copy_detect_target_helper "$fixture"
  copy_launcher_helper "$fixture"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/development-system/components/tiber/$(dirname "$binary_path")"
    cp "$ROOT/plugins/development-system/components/tiber/$binary_path" "$fixture/plugins/development-system/components/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/development-system/components/tiber/release-binaries.json")
  write_release_checksums "$fixture"
  printf '0000000000000000000000000000000000000000000000000000000000000000  dist/stale/tiber\n' >>"$fixture/plugins/development-system/components/tiber/release-binaries.sha256"

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"release-checksum-paths-mismatch"* ]]
}

@test "release build script reuses an already installed local toolchain" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/scripts"
  cp "$BUILD_ALL_SCRIPT" "$fixture/scripts/build-tiber-release-all.sh"
  cp "$ROOT/scripts/tiber-source-fingerprint.sh" "$fixture/scripts/tiber-source-fingerprint.sh"
  mkdir -p "$fixture/plugins/development-system/components/tiber"
  mkdir -p "$fixture/plugins/development-system/components/tiber/rust"
  printf '[workspace]\n' >"$fixture/plugins/development-system/components/tiber/rust/Cargo.toml"
  : >"$fixture/plugins/development-system/components/tiber/rust/Cargo.lock"
  mkdir -p "$fixture/plugins/development-system/components/tiber/rust/crates/fixture/src"
  printf '[package]\nname="fixture"\nversion="0.0.0"\n' > \
    "$fixture/plugins/development-system/components/tiber/rust/crates/fixture/Cargo.toml"
  : >"$fixture/plugins/development-system/components/tiber/rust/crates/fixture/src/lib.rs"
  cp "$ROOT/plugins/development-system/components/tiber/release-binaries.json" "$fixture/plugins/development-system/components/tiber/release-binaries.json"
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
  printf '%s\n' x86_64-unknown-linux-gnu
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
  printf '%s\n' "\${TIBER_SOURCE_FINGERPRINT:?}" >"\$target_dir/\$target/release/tiber"
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

  for target in x86_64-unknown-linux-gnu; do
    [ -x "$fixture/plugins/development-system/components/tiber/dist/$target/tiber" ]
  done
  [ -s "$fixture/plugins/development-system/components/tiber/release-binaries.sha256" ]

  rm -rf "$fixture"
  [ "$status" -eq 0 ]
}
