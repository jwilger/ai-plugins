#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  REPO="$BATS_TEST_TMPDIR/project"
  export XDG_RUNTIME_DIR="$BATS_TEST_TMPDIR/runtime"
  mkdir -p "$REPO"
  git -C "$REPO" init -q
}

@test "the root flake publishes the Codex VM command" {
  run bash -c '
    package="$(nix build --no-link --print-out-paths "$1#vm-codex" 2>/dev/null)" || exit
    cd "$2"
    env -i PATH=/nonexistent XDG_RUNTIME_DIR="$3" \
      "$package/bin/vm-codex" status --json
  ' _ "$ROOT" "$REPO" "$XDG_RUNTIME_DIR"

  [ "$status" -eq 0 ]
  [ "$(jq -r .project_root <<<"$output")" = "$REPO" ]
  [ "$(jq -r .running <<<"$output")" = false ]
}

@test "status reports a stable project identity without creating VM state" {
  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' status --json"

  [ "$status" -eq 0 ]
  [ "$(jq -r .schema_version <<<"$output")" -eq 1 ]
  [ "$(jq -r .project_root <<<"$output")" = "$REPO" ]
  [ "$(jq -r .state_dir <<<"$output")" = "$REPO/.codex-vm" ]
  [[ "$(jq -r .project_id <<<"$output")" =~ ^project-[0-9a-f]{16}$ ]]
  [ "$(jq -r .running <<<"$output")" = false ]
  [ ! -e "$REPO/.codex-vm" ]
}

@test "status does not mistake an unrelated reused PID for the VM" {
  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' status --json"
  [ "$status" -eq 0 ]
  project_id="$(jq -r .project_id <<<"$output")"
  runtime_dir="$XDG_RUNTIME_DIR/ai-plugins-codex-vm/$project_id"
  mkdir -p "$runtime_dir"

  sleep 60 &
  unrelated_pid=$!
  printf '%s\n' "$unrelated_pid" >"$runtime_dir/qemu.pid"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' status --json"
  kill "$unrelated_pid"

  [ "$status" -eq 0 ]
  [ "$(jq -r .running <<<"$output")" = false ]
}

@test "status tolerates a PID record disappearing during cleanup" {
  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' status --json"
  [ "$status" -eq 0 ]
  project_id="$(jq -r .project_id <<<"$output")"
  runtime_dir="$XDG_RUNTIME_DIR/ai-plugins-codex-vm/$project_id"
  mkdir -p "$runtime_dir/qemu.pid"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' status --json"

  [ "$status" -eq 0 ]
  [ "$(jq -r .running <<<"$output")" = false ]
}

@test "start dry-run reports the fixed VM boundary without creating state" {
  fake_bin="$BATS_TEST_TMPDIR/fake-bin"
  mkdir -p "$fake_bin"
  printf '#!/usr/bin/env bash\nexit 97\n' >"$fake_bin/nix"
  chmod +x "$fake_bin/nix"

  run env PATH="$fake_bin:$PATH" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --dry-run --json"

  [ "$status" -eq 0 ]
  [ "$(jq -r .schema_version <<<"$output")" -eq 1 ]
  [ "$(jq -r .resources.vcpus <<<"$output")" -eq 4 ]
  [ "$(jq -r .resources.memory_mib <<<"$output")" -eq 8192 ]
  [ "$(jq -r .resources.store_mib <<<"$output")" -eq 65536 ]
  [ "$(jq -r .resources.home_mib <<<"$output")" -eq 32768 ]
  [ "$(jq -r '.shares | length' <<<"$output")" -eq 1 ]
  [ "$(jq -r '.shares[0].source' <<<"$output")" = "$REPO" ]
  [ "$(jq -r '.shares[0].mount_point' <<<"$output")" = /work ]
  [ "$(jq -r '.shares[0].mode' <<<"$output")" = rw ]
  [ "$(jq -r .host_nix_evaluation <<<"$output")" = false ]
  [ ! -e "$REPO/.codex-vm" ]
  [ ! -e "$XDG_RUNTIME_DIR/ai-plugins-codex-vm" ]
}

@test "start rejects unknown project configuration fields" {
  printf '%s\n' '{"schemaVersion":1,"unexpected":true}' >"$REPO/codex-vm.json"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --dry-run --json"

  [ "$status" -eq 2 ]
  [[ "$output" == *"unknown configuration field: unexpected"* ]]
  [ ! -e "$REPO/.codex-vm" ]
  [ ! -e "$XDG_RUNTIME_DIR/ai-plugins-codex-vm" ]
}

@test "start rejects a dangling project configuration symlink" {
  ln -s "$REPO/missing-config.json" "$REPO/codex-vm.json"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --dry-run --json"

  [ "$status" -eq 2 ]
  [[ "$output" == *"project configuration must not be a symlink"* ]]
  [ ! -e "$REPO/.codex-vm" ]
  [ ! -e "$XDG_RUNTIME_DIR/ai-plugins-codex-vm" ]
}

@test "start requires a numeric schema version" {
  printf '%s\n' '{"schemaVersion":"1","shares":[]}' >"$REPO/codex-vm.json"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --dry-run --json"

  [ "$status" -eq 2 ]
  [[ "$output" == *"schemaVersion must be the number 1"* ]]
  [ ! -e "$REPO/.codex-vm" ]
  [ ! -e "$XDG_RUNTIME_DIR/ai-plugins-codex-vm" ]
}

@test "start requires shares to be an array" {
  printf '%s\n' '{"schemaVersion":1,"shares":false}' >"$REPO/codex-vm.json"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --dry-run --json"

  [ "$status" -eq 2 ]
  [[ "$output" == *"shares must be an array"* ]]
  [ ! -e "$REPO/.codex-vm" ]
  [ ! -e "$XDG_RUNTIME_DIR/ai-plugins-codex-vm" ]
}
