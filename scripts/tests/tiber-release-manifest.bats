#!/usr/bin/env bats

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd)"
  SCRIPT="$ROOT/scripts/check-tiber-release-manifest.sh"
  COMPLETE_SCRIPT="$ROOT/scripts/check-tiber-release-complete.sh"
  BUILD_ALL_SCRIPT="$ROOT/scripts/build-tiber-release-all.sh"
}

@test "real release manifest has an executable host binary" {
  run bash "$SCRIPT" "$ROOT"
  [ "$status" -eq 0 ]
}

@test "release manifest check fails when the host binary is missing" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"

  run bash "$SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-host-release-binary"* ]]
}

@test "release manifest check fails when the host binary is empty" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  host_path="$(bash -c '
    case "$(uname -s)-$(uname -m)" in
      Linux-x86_64) host_target="x86_64-unknown-linux-gnu" ;;
      Linux-aarch64 | Linux-arm64) host_target="aarch64-unknown-linux-gnu" ;;
      Darwin-x86_64) host_target="x86_64-apple-darwin" ;;
      Darwin-arm64 | Darwin-aarch64) host_target="aarch64-apple-darwin" ;;
    esac
    jq -r --arg target "$host_target" ".binaries[] | select(.target == \$target) | .path" "$0"
  ' "$ROOT/plugins/tiber/release-binaries.json")"
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
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/tiber/$(dirname "$binary_path")"
    printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/tiber/$binary_path"
    chmod +x "$fixture/plugins/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/tiber/release-binaries.json")

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -eq 0 ]
}

@test "complete release check fails when any target binary is missing" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  while IFS= read -r binary_path; do
    if [ "$binary_path" = "dist/aarch64-apple-darwin/tiber" ]; then
      continue
    fi
    mkdir -p "$fixture/plugins/tiber/$(dirname "$binary_path")"
    printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/tiber/$binary_path"
    chmod +x "$fixture/plugins/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/tiber/release-binaries.json")

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"missing-release-binary target=aarch64-apple-darwin"* ]]
}

@test "complete release check fails when any target binary is empty" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/plugins/tiber"
  cp "$ROOT/plugins/tiber/release-binaries.json" "$fixture/plugins/tiber/release-binaries.json"
  while IFS= read -r binary_path; do
    mkdir -p "$fixture/plugins/tiber/$(dirname "$binary_path")"
    if [ "$binary_path" != "dist/aarch64-apple-darwin/tiber" ]; then
      printf '#!/usr/bin/env sh\nexit 0\n' >"$fixture/plugins/tiber/$binary_path"
    else
      touch "$fixture/plugins/tiber/$binary_path"
    fi
    chmod +x "$fixture/plugins/tiber/$binary_path"
  done < <(jq -r '.binaries[].path' "$ROOT/plugins/tiber/release-binaries.json")

  run bash "$COMPLETE_SCRIPT" "$fixture"

  rm -rf "$fixture"
  [ "$status" -ne 0 ]
  [[ "$output" == *"invalid-release-binary target=aarch64-apple-darwin"* ]]
}

@test "release build script reuses an already installed local toolchain" {
  fixture="$(mktemp -d)"
  mkdir -p "$fixture/scripts"
  cp "$BUILD_ALL_SCRIPT" "$fixture/scripts/build-tiber-release-all.sh"
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

  rm -rf "$fixture"
  [ "$status" -eq 0 ]
}
