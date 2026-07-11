#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/.." && pwd -P)"
  SHELL_DIR="$BATS_TEST_TMPDIR/emc-devshell"
  NIX_BIN_DIR="$(dirname "$(command -v nix)")"
  mkdir -p "$SHELL_DIR"
}

run_emc_devshell() {
  run bash -c '
    cd "$1"
    nix develop --ignore-environment "$2" -c bash -c '\''
      printf "CARGO_HOME=%s\\n" "$CARGO_HOME"
      printf "CARGO_INSTALL_ROOT=%s\\n" "$CARGO_INSTALL_ROOT"
      printf "EMC=%s\\n" "$(command -v emc)"
      cargo install --list | sed -n '"'"'1s/^/EMC_INSTALL=/p'"'"'
    '\''
  ' _ "$SHELL_DIR" "$ROOT"
}

@test "development shell installs and reuses pinned project-local EMC" {
  run_emc_devshell

  [ "$status" -eq 0 ]
  [[ "$output" == *"Installing EMC 0.1.13"* ]]
  [[ "$output" == *"CARGO_HOME=$SHELL_DIR/.dependencies/cargo"* ]]
  [[ "$output" == *"CARGO_INSTALL_ROOT=$SHELL_DIR/.dependencies/cargo"* ]]
  [[ "$output" == *"EMC=$SHELL_DIR/.dependencies/cargo/bin/emc"* ]]
  [[ "$output" == *"EMC_INSTALL=emc v0.1.13:"* ]]

  run_emc_devshell

  [ "$status" -eq 0 ]
  [[ "$output" == *"EMC 0.1.13 is already installed"* ]]
}

@test "cargo-installed EMC serves the MCP initialize request" {
  run_emc_devshell
  [ "$status" -eq 0 ]

  run bash -c '
    cd "$1"
    printf "%s\\n" '\''{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2025-06-18","capabilities":{},"clientInfo":{"name":"emc-check","version":"0.0.0"}}}'\'' |
      timeout 30s nix develop --ignore-environment "$2" -c emc mcp stdio
  ' _ "$SHELL_DIR" "$ROOT"

  [ "$status" -eq 0 ]
  [[ "$output" == *'"name":"emc"'* ]]
  [[ "$output" == *'"version":"0.1.13"'* ]]
}

@test "CI devshell skips EMC installation" {
  local ci_shell_dir="$BATS_TEST_TMPDIR/emc-devshell-ci"

  mkdir -p "$ci_shell_dir"

  run bash -c '
    cd "$1"
    env -i CI=true HOME="$HOME" PATH="$2:/bin:/usr/bin" \
      nix develop "$3" -c bash -c '\''
        test ! -x "$CARGO_INSTALL_ROOT/bin/emc"
      '\''
  ' _ "$ci_shell_dir" "$NIX_BIN_DIR" "$ROOT"

  [ "$status" -eq 0 ]
  [[ "$output" != *"Installing EMC"* ]]
}

@test "development shell explains an EMC installation failure" {
  local fake_cargo="$SHELL_DIR/.dependencies/cargo/bin/cargo"

  mkdir -p "$(dirname "$fake_cargo")"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'if [ "$1" = "install" ] && [ "$2" = "--list" ]; then exit 0; fi' \
    'exit 72' >"$fake_cargo"
  chmod +x "$fake_cargo"

  run bash -c '
    cd "$1"
    nix develop --ignore-environment "$2" -c true
  ' _ "$SHELL_DIR" "$ROOT"

  [ "$status" -ne 0 ]
  [[ "$output" == *"emc.install_failed"* ]]
}

@test "EMC check is excluded from the CI Bats target" {
  run just --dry-run bats

  [ "$status" -eq 0 ]
  [[ "$output" != *"tests/emc-devshell.bats"* ]]

  run just --dry-run emc-check

  [ "$status" -eq 0 ]
  [[ "$output" == *"bats tests/emc-devshell.bats"* ]]
}
