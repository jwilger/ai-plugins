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
  package="$(nix build --no-link --print-out-paths "$ROOT#codex-vm-tools" 2>/dev/null)"

  run bash -c '
    package="$1"
    cd "$2"
    env -i PATH=/nonexistent XDG_RUNTIME_DIR="$3" \
      "$package/bin/vm-codex" status --json
  ' _ "$package" "$REPO" "$XDG_RUNTIME_DIR"

  [ "$status" -eq 0 ]
  [ "$(jq -r .project_root <<<"$output")" = "$REPO" ]
  [ "$(jq -r .running <<<"$output")" = false ]
  [ -x "$package/bin/vm-shell" ]

  run env -i PATH=/nonexistent "$package/bin/vm-codex" runtime --json
  [ "$status" -eq 0 ]
  [ "$(jq -r .host_nix_evaluation <<<"$output")" = false ]
  [[ "$(jq -r .tools.runner <<<"$output")" = /nix/store/*/bin/microvm-run ]]
  [[ "$(jq -r .tools.virtiofsd_run <<<"$output")" = /nix/store/*/bin/virtiofsd-run ]]

  run bash -c "cd '$REPO' && env -i PATH=/nonexistent XDG_RUNTIME_DIR='$XDG_RUNTIME_DIR' '$package/bin/vm-shell' --dry-run --json"
  [ "$status" -eq 0 ]
  [ "$(jq -c .guest_command <<<"$output")" = '["/bin/bash","-l"]' ]
  [ "$(jq -r .project_root <<<"$output")" = "$REPO" ]
  [ ! -e "$REPO/.codex-vm" ]
}

@test "the root flake owns the Codex MicroVM definition" {
  run bash -c '
    ref="$1#nixosConfigurations.codex-vm.config.microvm"
    hypervisor="$(nix eval --raw "$ref.hypervisor" 2>/dev/null)" || exit
    vcpu="$(nix eval --json "$ref.vcpu" 2>/dev/null)" || exit
    mem="$(nix eval --json "$ref.mem" 2>/dev/null)" || exit
    volumes="$(nix eval --json "$ref.volumes" 2>/dev/null)" || exit
    shares="$(nix eval --json "$ref.shares" 2>/dev/null)" || exit
    runtime_args="$(nix eval --raw "$ref.extraArgsScript" 2>/dev/null)" || exit
    ssh="$(nix eval --json "$1#nixosConfigurations.codex-vm.config.services.openssh.enable" 2>/dev/null)" || exit
    runner="$(nix eval --raw "$1#packages.x86_64-linux.codex-vm-runner.drvPath" 2>/dev/null)" || exit
    jq -cn \
      --arg hypervisor "$hypervisor" \
      --argjson vcpu "$vcpu" \
      --argjson mem "$mem" \
      --argjson volumes "$volumes" \
      --argjson shares "$shares" \
      --arg runtime_args "$runtime_args" \
      --argjson ssh "$ssh" \
      --arg runner "$runner" \
      "{\$hypervisor, \$vcpu, \$mem, \$volumes, \$shares, \$runtime_args, \$ssh, \$runner}"
  ' _ "$ROOT"

  [ "$status" -eq 0 ]
  [ "$(jq -r .hypervisor <<<"$output")" = qemu ]
  [ "$(jq -r .vcpu <<<"$output")" -eq 4 ]
  [ "$(jq -r .mem <<<"$output")" -eq 8192 ]
  [ "$(jq -r '.volumes | length' <<<"$output")" -eq 2 ]
  [ "$(jq -r '.volumes[] | select(.mountPoint == "/nix/.rw-store") | .size' <<<"$output")" -eq 65536 ]
  [ "$(jq -r '.volumes[] | select(.mountPoint == "/home/codex") | .size' <<<"$output")" -eq 32768 ]
  [ "$(jq -r '.shares[] | select(.mountPoint == "/work") | .source' <<<"$output")" = work-export ]
  [ "$(jq -r '.shares[] | select(.mountPoint == "/run/host-keys") | .readOnly' <<<"$output")" = true ]
  [[ "$(jq -r .runtime_args <<<"$output")" = /nix/store/*-codex-qemu-runtime-args ]]
  [ "$(jq -r .ssh <<<"$output")" = true ]
  [[ "$(jq -r .runner <<<"$output")" = /nix/store/*-microvm-qemu-codex-vm.drv ]]
}

@test "the root flake runtime script safely forwards SSH" {
  runtime_args="$(nix build --no-link --print-out-paths "$ROOT#codex-vm-qemu-runtime-args" 2>/dev/null)"

  run env -i CODEX_VM_SSH_PORT=43210 "$runtime_args"
  [ "$status" -eq 0 ]
  [ "$output" = "-nic user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:43210-:22" ]

  run env -i CODEX_VM_SSH_PORT='43210,hostfwd=tcp:0.0.0.0:1-:1' "$runtime_args"
  [ "$status" -eq 2 ]

  run env -i CODEX_VM_SSH_PORT=22 "$runtime_args"
  [ "$status" -eq 2 ]
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
