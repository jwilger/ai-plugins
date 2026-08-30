#!/usr/bin/env bats

bats_require_minimum_version 1.5.0

setup() {
  ROOT="$(cd "$BATS_TEST_DIRNAME/../.." && pwd -P)"
  REPO="$BATS_TEST_TMPDIR/project"
  export XDG_RUNTIME_DIR="$BATS_TEST_TMPDIR/runtime"
  mkdir -p "$REPO"
  git -C "$REPO" init -q
}

make_fake_vm_runtime() {
  FAKE_VM_BIN="$BATS_TEST_TMPDIR/lifecycle-bin"
  mkdir -p "$FAKE_VM_BIN"
  printf '#!/usr/bin/env bash\nexec sleep 60\n' >"$FAKE_VM_BIN/runner"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'exec python3 -c '\''import socket,time; tags=("work","host-keys","bootstrap","extra-ro-0","extra-rw-0","extra-ro-1","extra-rw-1","extra-ro-2","extra-rw-2","extra-ro-3","extra-rw-3"); sockets=[]; [(sockets.append(socket.socket(socket.AF_UNIX)), sockets[-1].bind(f"codex-vm-virtiofs-{tag}.sock")) for tag in tags]; time.sleep(60)'\''' \
    >"$FAKE_VM_BIN/virtiofsd-run"
  printf '%s\n' \
    '#!/usr/bin/env bash' \
    'printf "%s\\n" "$@" >"$FAKE_BWRAP_ARGS"' \
    'printf "%s\\0" "$@" >"$FAKE_BWRAP_ARGS_NUL"' \
    'arguments=("$@")' \
    'runtime_dir=' \
    'for ((index = 0; index < ${#arguments[@]} - 2; index++)); do [ "${arguments[$index]}" = --bind ] && [ "${arguments[$((index + 2))]}" = /runtime ] && runtime_dir="${arguments[$((index + 1))]}"; done' \
    'for ((index = 0; index < ${#arguments[@]}; index++)); do if [ "${arguments[$index]}" = --chdir ]; then directory="${arguments[$((index + 1))]}"; [ "$directory" = /runtime ] && directory="$runtime_dir"; cd "$directory"; fi; done' \
    'exec "${arguments[-1]}"' \
    >"$FAKE_VM_BIN/bwrap"
  printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$@" >"$FAKE_SSH_ARGS"\nexit 0\n' >"$FAKE_VM_BIN/ssh"
  printf '#!/usr/bin/env bash\nexit 97\n' >"$FAKE_VM_BIN/nix"
  printf '#!/usr/bin/env bash\nprintf "%%s\\n" "$@" >"$FAKE_QMP_ARGS"\n' >"$FAKE_VM_BIN/qmp-forward"
  chmod +x "$FAKE_VM_BIN"/*
  export PATH="$FAKE_VM_BIN:$PATH"
  export CODEX_VM_RUNNER="$FAKE_VM_BIN/runner"
  export CODEX_VM_VIRTIOFSD_RUN="$FAKE_VM_BIN/virtiofsd-run"
  export CODEX_VM_BWRAP="$FAKE_VM_BIN/bwrap"
  export CODEX_VM_FLOCK="$(command -v flock)"
  export CODEX_VM_PYTHON="$(command -v python3)"
  export CODEX_VM_QMP_FORWARD="$FAKE_VM_BIN/qmp-forward"
  export CODEX_VM_SSH="$FAKE_VM_BIN/ssh"
  export CODEX_VM_SSH_KEYGEN="$(command -v ssh-keygen)"
  export FAKE_BWRAP_ARGS="$BATS_TEST_TMPDIR/bwrap.args"
  export FAKE_BWRAP_ARGS_NUL="$BATS_TEST_TMPDIR/bwrap.args.nul"
  export FAKE_SSH_ARGS="$BATS_TEST_TMPDIR/ssh.args"
  export FAKE_QMP_ARGS="$BATS_TEST_TMPDIR/qmp.args"
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

@test "the root flake publishes a NixOS host integration module" {
  run --separate-stderr nix eval --json --impure --expr '
    let
      flake = builtins.getFlake (toString ./.);
      pkgs = flake.inputs.nixpkgs.legacyPackages.x86_64-linux;
      evaluated = flake.nixosModules.codex-vm-host {
        config.programs.aiPlugins.codexVm = {
          enable = true;
          package = pkgs.hello;
        };
        lib = pkgs.lib;
        inherit pkgs;
      };
      body = evaluated.config.content;
    in {
      enabled = evaluated.config.condition;
      packages = map (package: package.name) body.environment.systemPackages;
      assertions = map (item: item.assertion) body.assertions;
      enable_default = evaluated.options.programs.aiPlugins.codexVm.enable.default;
    }
  '

  if [ "$status" -ne 0 ]; then
    printf '%s\n' "$stderr" >&3
    false
  fi
  [ "$(jq -r '[.packages[] | select(startswith("hello-"))] | length' <<<"$output")" -eq 1 ]
  [ "$(jq -r .enabled <<<"$output")" = true ]
  [ "$(jq -r '.assertions | all' <<<"$output")" = true ]
  [ "$(jq -r .enable_default <<<"$output")" = false ]
}

@test "the exact-revision installer atomically publishes stable host launchers" {
  installer="$ROOT/scripts/codex-vm/install-codex-vm"
  data_home="$BATS_TEST_TMPDIR/data"
  bin_dir="$BATS_TEST_TMPDIR/bin"
  first_tools="$BATS_TEST_TMPDIR/tools-a"
  second_tools="$BATS_TEST_TMPDIR/tools-b"
  first_revision=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  second_revision=bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
  mkdir -p "$first_tools/bin" "$second_tools/bin"
  printf '#!/usr/bin/env bash\nprintf "first:%%s\\n" "$1"\n' >"$first_tools/bin/vm-codex"
  printf '#!/usr/bin/env bash\nprintf "first-shell\\n"\n' >"$first_tools/bin/vm-shell"
  printf '#!/usr/bin/env bash\nprintf "second:%%s\\n" "$1"\n' >"$second_tools/bin/vm-codex"
  printf '#!/usr/bin/env bash\nprintf "second-shell\\n"\n' >"$second_tools/bin/vm-shell"
  chmod +x "$first_tools/bin/"* "$second_tools/bin/"*

  run env \
    CODEX_VM_SOURCE_REVISION="$first_revision" \
    CODEX_VM_INSTALL_TOOLS="$first_tools" \
    XDG_DATA_HOME="$data_home" \
    XDG_BIN_HOME="$bin_dir" \
    bash "$installer" install --revision "$first_revision"

  [ "$status" -eq 0 ]
  run "$bin_dir/vm-codex" status
  [ "$status" -eq 0 ]
  [ "$output" = first:status ]
  [ "$(readlink "$data_home/ai-plugins/codex-vm/current")" = "revisions/$first_revision" ]

  run env \
    CODEX_VM_SOURCE_REVISION="$second_revision" \
    CODEX_VM_INSTALL_TOOLS="$second_tools" \
    XDG_DATA_HOME="$data_home" \
    XDG_BIN_HOME="$bin_dir" \
    bash "$installer" install --revision "$second_revision"

  [ "$status" -eq 0 ]
  run "$bin_dir/vm-codex" status
  [ "$status" -eq 0 ]
  [ "$output" = second:status ]
  [ "$(readlink "$data_home/ai-plugins/codex-vm/current")" = "revisions/$second_revision" ]
  [ -L "$data_home/ai-plugins/codex-vm/revisions/$first_revision" ]
}

@test "the installer rejects mutable or mismatched source revisions" {
  installer="$ROOT/scripts/codex-vm/install-codex-vm"
  tools="$BATS_TEST_TMPDIR/tools"
  revision=aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa
  mkdir -p "$tools/bin"
  printf '#!/usr/bin/env bash\n' >"$tools/bin/vm-codex"
  printf '#!/usr/bin/env bash\n' >"$tools/bin/vm-shell"
  chmod +x "$tools/bin/"*

  run env CODEX_VM_SOURCE_REVISION=dirty CODEX_VM_INSTALL_TOOLS="$tools" \
    bash "$installer" install --revision "$revision"
  [ "$status" -eq 2 ]
  [[ "$output" == *"not built from an immutable exact Git revision"* ]]

  run env CODEX_VM_SOURCE_REVISION="$revision" CODEX_VM_INSTALL_TOOLS="$tools" \
    bash "$installer" install --revision bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb
  [ "$status" -eq 2 ]
  [[ "$output" == *"requested revision does not match"* ]]
}

@test "init creates isolated project SSH state without invoking Nix" {
  package="$(nix build --no-link --print-out-paths "$ROOT#codex-vm-tools" 2>/dev/null)"
  fake_bin="$BATS_TEST_TMPDIR/no-nix"
  mkdir -p "$fake_bin"
  printf '#!/usr/bin/env bash\nexit 97\n' >"$fake_bin/nix"
  chmod +x "$fake_bin/nix"

  run env PATH="$fake_bin:$PATH" bash -c "cd '$REPO' && '$package/bin/vm-codex' init --json"

  [ "$status" -eq 0 ]
  [ "$(jq -r .state_dir <<<"$output")" = "$REPO/.codex-vm" ]
  [ -f "$REPO/.codex-vm/ssh/client_ed25519" ]
  [ -f "$REPO/.codex-vm/ssh-keys/codex.pub" ]
  [ -f "$REPO/.codex-vm/ssh-keys/ssh_host_ed25519_key" ]
  [ "$(stat -c %a "$REPO/.codex-vm/ssh/client_ed25519")" = 600 ]
  [ "$(stat -c %a "$REPO/.codex-vm/ssh/known_hosts")" = 600 ]
  [[ "$(cat "$REPO/.codex-vm/ssh/known_hosts")" = project-*" "ssh-ed25519* ]]
  git -C "$ROOT" check-ignore -q .codex-vm/example
}

@test "init seeds only allowlisted host Codex preferences once" {
  package="$(nix build --no-link --print-out-paths "$ROOT#codex-vm-tools" 2>/dev/null)"
  host_home="$BATS_TEST_TMPDIR/host-home"
  mkdir -p "$host_home/.codex"
  cat >"$host_home/.codex/config.toml" <<'EOF'
model = "gpt-5.6-sol"
model_reasoning_effort = "high"
model_verbosity = "low"
personality = "pragmatic"
plan_mode_reasoning_effort = "xhigh"
web_search = "live"
file_opener = "none"
hide_agent_reasoning = true
forced_login_method = "api"
model_provider = "private-provider"

[history]
persistence = "save-all"

[mcp_servers.private]
env = { SECRET_TOKEN = "must-not-copy" }
EOF

  run env HOME="$host_home" bash -c "cd '$REPO' && '$package/bin/vm-codex' init"
  [ "$status" -eq 0 ]
  seed="$REPO/.codex-vm/bootstrap/preferences.toml"
  [ -f "$seed" ]
  [ "$(stat -c %a "$seed")" = 600 ]
  run python3 - "$seed" <<'PY'
import pathlib, sys, tomllib
path = pathlib.Path(sys.argv[1])
parsed = tomllib.loads(path.read_text())
assert parsed == {
    "model": "gpt-5.6-sol",
    "model_reasoning_effort": "high",
    "model_verbosity": "low",
    "personality": "pragmatic",
    "plan_mode_reasoning_effort": "xhigh",
    "web_search": "live",
    "file_opener": "none",
    "hide_agent_reasoning": True,
}
assert "must-not-copy" not in path.read_text()
PY
  [ "$status" -eq 0 ]

  sed -i 's/gpt-5.6-sol/gpt-5.6-terra/' "$host_home/.codex/config.toml"
  run env HOME="$host_home" bash -c "cd '$REPO' && '$package/bin/vm-codex' init"
  [ "$status" -eq 0 ]
  grep -q 'gpt-5.6-sol' "$seed"
  ! grep -q 'gpt-5.6-terra' "$seed"
}

@test "init snapshots the applicable envrc chain as private data without executing it" {
  package="$(nix build --no-link --print-out-paths "$ROOT#codex-vm-tools" 2>/dev/null)"
  parent_envrc="$BATS_TEST_TMPDIR/.envrc"
  execution_marker="$BATS_TEST_TMPDIR/envrc-was-executed"
  printf 'export PARENT_VALUE=parent\ntouch %q\n' "$execution_marker" >"$parent_envrc"
  printf 'source_up_if_exists\nexport PROJECT_VALUE=project\n' >"$REPO/.envrc"

  run env AMBIENT_ONLY=must-not-be-captured bash -c "cd '$REPO' && '$package/bin/vm-codex' init"
  [ "$status" -eq 0 ]
  [ ! -e "$execution_marker" ]
  manifest="$REPO/.codex-vm/bootstrap/envrc/manifest.json"
  [ -f "$manifest" ]
  [ "$(stat -c %a "$manifest")" = 600 ]
  [ "$(jq -r .projectRoot "$manifest")" = "$REPO" ]
  [ "$(jq -r '.files | length' "$manifest")" -eq 2 ]
  jq -e --arg path "$parent_envrc" '.files[] | select(.path == $path and .kind == "ancestor")' "$manifest" >/dev/null
  jq -e --arg path "$REPO/.envrc" '.files[] | select(.path == $path and .kind == "project")' "$manifest" >/dev/null
  while IFS= read -r staged; do
    [ -f "$REPO/.codex-vm/bootstrap/envrc/files/$staged" ]
    [ "$(stat -c %a "$REPO/.codex-vm/bootstrap/envrc/files/$staged")" = 600 ]
  done < <(jq -r '.files[].staged' "$manifest")
  ! grep -R -q 'AMBIENT_ONLY' "$REPO/.codex-vm/bootstrap/envrc"
}

@test "init rejects agent-controlled VM state symlinks" {
  package="$(nix build --no-link --print-out-paths "$ROOT#codex-vm-tools" 2>/dev/null)"
  outside="$BATS_TEST_TMPDIR/outside"
  mkdir -p "$outside"
  ln -s "$outside" "$REPO/.codex-vm"

  run bash -c "cd '$REPO' && '$package/bin/vm-codex' init --json"

  [ "$status" -eq 2 ]
  [[ "$output" == *"VM state directories must not be symlinks"* ]]
  [ -z "$(find "$outside" -mindepth 1 -print -quit)" ]
}

@test "init rejects nested symlinks and hardlinks in VM state" {
  package="$(nix build --no-link --print-out-paths "$ROOT#codex-vm-tools" 2>/dev/null)"
  mkdir -p "$REPO/.codex-vm"
  ln -s "$BATS_TEST_TMPDIR" "$REPO/.codex-vm/ssh"

  run bash -c "cd '$REPO' && '$package/bin/vm-codex' init"
  [ "$status" -eq 2 ]
  [[ "$output" == *"VM state directories must not be symlinks"* ]]

  rm "$REPO/.codex-vm/ssh"
  run bash -c "cd '$REPO' && '$package/bin/vm-codex' init"
  [ "$status" -eq 0 ]
  cp "$REPO/.codex-vm/ssh/client_ed25519.pub" "$BATS_TEST_TMPDIR/hardlinked.pub"
  rm "$REPO/.codex-vm/ssh/client_ed25519.pub"
  ln "$BATS_TEST_TMPDIR/hardlinked.pub" "$REPO/.codex-vm/ssh/client_ed25519.pub"

  run bash -c "cd '$REPO' && '$package/bin/vm-codex' init"
  [ "$status" -eq 2 ]
  [[ "$output" == *"VM state files must not be hard-linked"* ]]
}

@test "init repairs missing or mismatched public keys from private keys" {
  package="$(nix build --no-link --print-out-paths "$ROOT#codex-vm-tools" 2>/dev/null)"
  run bash -c "cd '$REPO' && '$package/bin/vm-codex' init"
  [ "$status" -eq 0 ]
  rm "$REPO/.codex-vm/ssh/client_ed25519.pub"
  printf '%s\n' 'ssh-ed25519 stale' >"$REPO/.codex-vm/ssh-keys/ssh_host_ed25519_key.pub"

  run bash -c "cd '$REPO' && '$package/bin/vm-codex' init"
  [ "$status" -eq 0 ]
  [ "$(cat "$REPO/.codex-vm/ssh/client_ed25519.pub")" = "$(ssh-keygen -y -f "$REPO/.codex-vm/ssh/client_ed25519")" ]
  [ "$(cat "$REPO/.codex-vm/ssh-keys/ssh_host_ed25519_key.pub")" = "$(ssh-keygen -y -f "$REPO/.codex-vm/ssh-keys/ssh_host_ed25519_key")" ]
}

@test "init ignores stale interrupted temporary state" {
  package="$(nix build --no-link --print-out-paths "$ROOT#codex-vm-tools" 2>/dev/null)"
  mkdir -p "$REPO/.codex-vm/.init.stale"
  printf '%s\n' stale >"$REPO/.codex-vm/.init.stale/client_ed25519"

  run bash -c "cd '$REPO' && '$package/bin/vm-codex' init"

  [ "$status" -eq 0 ]
  [ -f "$REPO/.codex-vm/ssh/client_ed25519" ]
}

@test "the root flake owns the Codex MicroVM definition" {
  run --separate-stderr nix eval --json --impure --expr '
    let
      flake = builtins.getFlake (toString ./.);
      cfg = flake.nixosConfigurations.codex-vm.config;
    in {
      hypervisor = cfg.microvm.hypervisor;
      inherit (cfg.microvm) vcpu mem volumes shares;
      interfaces = map (interface: {
        inherit (interface) type id mac;
      }) cfg.microvm.interfaces;
      ssh = cfg.services.openssh.enable;
      ssh_firewall = cfg.services.openssh.openFirewall;
      sudo_password = cfg.security.sudo.wheelNeedsPassword;
      blacklisted_modules = cfg.boot.blacklistedKernelModules;
      system_packages = map (package: package.name) cfg.environment.systemPackages;
      egress_rules = cfg.networking.nftables.tables.codex-vm-egress.content;
      host_keys = cfg.services.openssh.hostKeys;
      bootstrap_script = cfg.systemd.services.codex-vm-bootstrap.script;
      bootstrap_before = cfg.systemd.services.codex-vm-bootstrap.before;
      environment_service = cfg.systemd.services.codex-vm-environment.serviceConfig;
      runner = flake.packages.x86_64-linux.codex-vm-runner.drvPath;
    }
  '

  if [ "$status" -ne 0 ]; then
    printf '%s\n' "$stderr" >&3
    false
  fi
  [ "$(jq -r .hypervisor <<<"$output")" = qemu ]
  [ "$(jq -r .vcpu <<<"$output")" -eq 4 ]
  [ "$(jq -r .mem <<<"$output")" -eq 8192 ]
  [ "$(jq -r '.volumes | length' <<<"$output")" -eq 2 ]
  [ "$(jq -r '.volumes[] | select(.mountPoint == "/nix/.rw-store") | .size' <<<"$output")" -eq 65536 ]
  [ "$(jq -r '.volumes[] | select(.mountPoint == "/home/codex") | .size' <<<"$output")" -eq 32768 ]
  [ "$(jq -r '.shares[] | select(.mountPoint == "/work") | .source' <<<"$output")" = work-export ]
  [ "$(jq -r '.shares[] | select(.mountPoint == "/run/host-keys") | .readOnly' <<<"$output")" = true ]
  [ "$(jq -r '.shares[] | select(.mountPoint == "/run/codex-vm-bootstrap") | .readOnly' <<<"$output")" = true ]
  [ "$(jq -r '.interfaces[0] | [.type, .id, .mac] | @tsv' <<<"$output")" = $'user\tcodexnet\t02:00:00:00:00:01' ]
  [ "$(jq -r .ssh <<<"$output")" = true ]
  [ "$(jq -r .ssh_firewall <<<"$output")" = true ]
  [ "$(jq -r .sudo_password <<<"$output")" = false ]
  [ "$(jq -r '[.blacklisted_modules | index("kvm"), index("kvm-intel"), index("kvm-amd")] | all(. != null)' <<<"$output")" = true ]
  [ "$(jq -r '[.system_packages[] | select(test("^codex-[0-9]"))] | length' <<<"$output")" -eq 1 ]
  [ "$(jq -r '[.system_packages[] | select(. == "codex-vm-enter")] | length' <<<"$output")" -eq 1 ]
  [ "$(jq -r '[.system_packages[] | select(startswith("direnv-"))] | length' <<<"$output")" -eq 1 ]
  egress_rules="$(jq -r .egress_rules <<<"$output")"
  [[ "$egress_rules" == *"ct state established,related accept"* ]]
  [[ "$egress_rules" == *"10.0.2.3 udp dport 53 accept"* ]]
  [[ "$egress_rules" == *"10.0.2.3 tcp dport 53 accept"* ]]
  [[ "$egress_rules" == *"fec0::3 udp dport 53 accept"* ]]
  [[ "$egress_rules" == *"fec0::3 tcp dport 53 accept"* ]]
  [[ "$(jq -r .egress_rules <<<"$output")" == *"10.0.0.0/8"* ]]
  [[ "$(jq -r .egress_rules <<<"$output")" == *"169.254.0.0/16"* ]]
  [[ "$egress_rules" == *"fc00::/7"* ]]
  private_v4_line="$(grep -nF 'ip daddr { 10.0.0.0/8' <<<"$egress_rules" | cut -d: -f1)"
  private_v6_line="$(grep -nF 'ip6 daddr { fc00::/7' <<<"$egress_rules" | cut -d: -f1)"
  for rule in \
    'ct state established,related accept' \
    'ip daddr 10.0.2.3 udp dport 53 accept' \
    'ip daddr 10.0.2.3 tcp dport 53 accept'; do
    [ "$(grep -nF "$rule" <<<"$egress_rules" | cut -d: -f1)" -lt "$private_v4_line" ]
  done
  for rule in \
    'ip6 daddr fec0::3 udp dport 53 accept' \
    'ip6 daddr fec0::3 tcp dport 53 accept'; do
    [ "$(grep -nF "$rule" <<<"$egress_rules" | cut -d: -f1)" -lt "$private_v6_line" ]
  done
  [ "$(jq -r .host_keys[0].path <<<"$output")" = /run/host-keys/ssh_host_ed25519_key ]
  [[ "$(jq -r .bootstrap_script <<<"$output")" == *"preferences.toml"* ]]
  [[ "$(jq -r .bootstrap_script <<<"$output")" == *"codex_home=/home/codex/.codex"* ]]
  [[ "$(jq -r .bootstrap_script <<<"$output")" == *"chown codex:codex /home/codex"* ]]
  [[ "$(jq -r .bootstrap_script <<<"$output")" == *'"$codex_home/config.toml"'* ]]
  [[ "$(jq -r .bootstrap_script <<<"$output")" == *"/home/codex/.ssh/authorized_keys"* ]]
  [ "$(jq -r '.bootstrap_before | index("sshd.service") != null' <<<"$output")" = true ]
  [[ "$(jq -r .environment_service.ExecStart <<<"$output")" = /nix/store/*-codex-vm-refresh-environment ]]
  [[ "$(jq -r .runner <<<"$output")" = /nix/store/*-microvm-qemu-codex-vm.drv ]]
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

@test "start and stop manage the same VM process without invoking Nix" {
  make_fake_vm_runtime

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"

  [ "$status" -eq 0 ]
  qemu_pid="$(jq -r .pid <<<"$output")"
  ssh_port="$(jq -r .ssh_port <<<"$output")"
  kill -0 "$qemu_pid"
  [[ "$ssh_port" =~ ^[0-9]+$ ]]
  [ "$(sed -n '1p' "$FAKE_QMP_ARGS")" = "$XDG_RUNTIME_DIR/ai-plugins-codex-vm/$(jq -r .project_id <<<"$output")/codex-vm.sock" ]
  [ "$(sed -n '2p' "$FAKE_QMP_ARGS")" = "$ssh_port" ]
  grep -Fx -- '--ro-bind' "$FAKE_BWRAP_ARGS"
  grep -Fx -- '/nix/store' "$FAKE_BWRAP_ARGS"
  grep -Fx -- '/etc/passwd' "$FAKE_BWRAP_ARGS"
  grep -Fx -- '/etc/group' "$FAKE_BWRAP_ARGS"
  grep -Fx -- '--tmpfs' "$FAKE_BWRAP_ARGS"
  grep -Fx -- '/runtime/work-export/.codex-vm' "$FAKE_BWRAP_ARGS"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' status --json"
  [ "$status" -eq 0 ]
  [ "$(jq -r .running <<<"$output")" = true ]
  [ "$(jq -r .ssh_port <<<"$output")" = "$ssh_port" ]

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' stop --json"
  [ "$status" -eq 0 ]
  [ "$(jq -r .running <<<"$output")" = false ]

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' status --json"
  [ "$status" -eq 0 ]
  [ "$(jq -r .running <<<"$output")" = false ]

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"
  [ "$status" -eq 0 ]
  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' stop --json"
  [ "$status" -eq 0 ]
}

@test "start rejects agent-controlled runtime log symlinks" {
  make_fake_vm_runtime
  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' init"
  [ "$status" -eq 0 ]
  outside="$BATS_TEST_TMPDIR/outside.log"
  printf '%s\n' preserved >"$outside"
  ln -s "$outside" "$REPO/.codex-vm/qemu.log"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"

  [ "$status" -eq 2 ]
  [[ "$output" == *"VM runtime files must not be symlinks"* ]]
  [ "$(cat "$outside")" = preserved ]
}

@test "start retries a runner that loses its first port race" {
  make_fake_vm_runtime
  attempts="$BATS_TEST_TMPDIR/runner-attempts"
  printf '#!/usr/bin/env bash\nprintf x >>"%s"\n[ "$(wc -c <"%s")" -gt 1 ] || exit 1\nexec sleep 60\n' \
    "$attempts" "$attempts" >"$FAKE_VM_BIN/runner"
  chmod +x "$FAKE_VM_BIN/runner"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"

  [ "$status" -eq 0 ]
  [ "$(wc -c <"$attempts")" -eq 2 ]
  [ "$(jq -r .running <<<"$output")" = true ]
  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' stop"
  [ "$status" -eq 0 ]
}

@test "start rejects a running VM with missing SSH port state" {
  make_fake_vm_runtime
  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"
  [ "$status" -eq 0 ]
  rm "$XDG_RUNTIME_DIR/ai-plugins-codex-vm/$(jq -r .project_id <<<"$output")/ssh.port"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"
  [ "$status" -eq 2 ]
  [[ "$output" == *"running VM has invalid SSH port state"* ]]

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' stop"
  [ "$status" -eq 0 ]
}

@test "Codex and shell connections use only project SSH identity" {
  make_fake_vm_runtime

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' shell"
  [ "$status" -eq 0 ]
  grep -Fx -- '-F' "$FAKE_SSH_ARGS"
  grep -Fx -- '/dev/null' "$FAKE_SSH_ARGS"
  grep -Fx -- 'IdentityAgent=none' "$FAKE_SSH_ARGS"
  grep -Fx -- 'IdentitiesOnly=yes' "$FAKE_SSH_ARGS"
  project_id="$(cd "$REPO" && "$ROOT/scripts/codex-vm/vm-codex" status --json | jq -r .project_id)"
  grep -Fx -- "HostKeyAlias=$project_id" "$FAKE_SSH_ARGS"
  grep -F -- "$REPO/.codex-vm/ssh/client_ed25519" "$FAKE_SSH_ARGS"
  grep -Fx -- 'exec codex-vm-enter shell' "$FAKE_SSH_ARGS"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' codex"
  [ "$status" -eq 0 ]
  grep -Fx -- 'exec codex-vm-enter codex --yolo' "$FAKE_SSH_ARGS"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' remote-control start"
  [ "$status" -eq 0 ]
  grep -Fx -- 'exec codex-vm-enter codex --yolo remote-control start --json' "$FAKE_SSH_ARGS"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' login"
  [ "$status" -eq 0 ]
  grep -Fx -- 'exec codex-vm-enter codex --yolo login --device-auth' "$FAKE_SSH_ARGS"

  for action in pair stop; do
    run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' remote-control '$action'"
    [ "$status" -eq 0 ]
    grep -Fx -- "exec codex-vm-enter codex --yolo remote-control $action --json" "$FAKE_SSH_ARGS"
  done

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' stop"
  [ "$status" -eq 0 ]
}

@test "start rolls back both processes when strict SSH never becomes ready" {
  make_fake_vm_runtime
  printf '#!/usr/bin/env bash\nsleep 0.3\nexit 1\n' >"$FAKE_VM_BIN/runner"
  printf '#!/usr/bin/env bash\nexit 1\n' >"$FAKE_VM_BIN/ssh"
  chmod +x "$FAKE_VM_BIN/runner" "$FAKE_VM_BIN/ssh"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"

  [ "$status" -eq 2 ]
  [[ "$output" == *"VM did not become reachable over project SSH"* || "$output" == *"QEMU failed to claim"* ]]
  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' status --json"
  [ "$status" -eq 0 ]
  [ "$(jq -r .running <<<"$output")" = false ]
  project_id="$(jq -r .project_id <<<"$output")"
  [ ! -e "$XDG_RUNTIME_DIR/ai-plugins-codex-vm/$project_id/qemu.pid" ]
  [ ! -e "$XDG_RUNTIME_DIR/ai-plugins-codex-vm/$project_id/virtiofsd.pid" ]
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

@test "start dry-run accepts only owner-authorized canonical extra shares" {
  host_home="$BATS_TEST_TMPDIR/host-home"
  read_source="$BATS_TEST_TMPDIR/reference"
  write_source="$BATS_TEST_TMPDIR/output"
  mkdir -p "$host_home/.config/ai-plugins" "$read_source" "$write_source"
  project_id="$(cd "$REPO" && "$ROOT/scripts/codex-vm/vm-codex" status --json | jq -r .project_id)"

  jq -n \
    --arg project_id "$project_id" \
    --arg read_source "$read_source" \
    --arg write_source "$write_source" \
    '{schemaVersion: 1, projects: {($project_id): [
      {source: $read_source, mountPoint: "/mnt/reference", mode: "ro"},
      {source: $write_source, mountPoint: "/mnt/output", mode: "rw"}
    ]}}' >"$host_home/.config/ai-plugins/codex-vm-shares.json"
  jq -n \
    --arg read_source "$read_source" \
    --arg write_source "$write_source" \
    '{schemaVersion: 1, shares: [
      {source: $read_source, mountPoint: "/mnt/reference", mode: "ro"},
      {source: $write_source, mountPoint: "/mnt/output", mode: "rw"}
    ]}' >"$REPO/codex-vm.json"

  run env HOME="$host_home" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --dry-run --json"

  [ "$status" -eq 0 ]
  [ "$(jq -r '.shares[] | select(.mount_point == "/mnt/reference") | .mode' <<<"$output")" = ro ]
  [ "$(jq -r '.shares[] | select(.mount_point == "/mnt/output") | .mode' <<<"$output")" = rw ]
  [ ! -e "$REPO/.codex-vm" ]

  for mismatch in \
    '{"source":"READ_SOURCE","mountPoint":"/mnt/reference","mode":"rw"}' \
    '{"source":"WRITE_SOURCE","mountPoint":"/mnt/reference","mode":"ro"}' \
    '{"source":"READ_SOURCE","mountPoint":"/mnt/renamed","mode":"ro"}'; do
    mismatch="${mismatch//READ_SOURCE/$read_source}"
    mismatch="${mismatch//WRITE_SOURCE/$write_source}"
    jq -n \
      --arg project_id "$project_id" \
      --arg write_source "$write_source" \
      --argjson mismatch "$mismatch" \
      '{schemaVersion: 1, projects: {($project_id): [
        $mismatch,
        {source: $write_source, mountPoint: "/mnt/output", mode: "rw"}
      ]}}' \
      >"$host_home/.config/ai-plugins/codex-vm-shares.json"

    run env HOME="$host_home" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --dry-run --json"

    [ "$status" -eq 2 ]
    [[ "$output" == *"extra share is not authorized for this project"* ]]
  done

  ln -s "$read_source" "$BATS_TEST_TMPDIR/reference-link"
  jq -n \
    --arg source "$BATS_TEST_TMPDIR/reference-link" \
    '{schemaVersion: 1, shares: [{source: $source, mountPoint: "/mnt/reference", mode: "ro"}]}' \
    >"$REPO/codex-vm.json"
  run env HOME="$host_home" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --dry-run --json"

  [ "$status" -eq 2 ]
  [[ "$output" == *"source must be a canonical non-root directory"* ]]
  [ ! -e "$REPO/.codex-vm" ]
}

@test "start reports malformed share modes without a traceback" {
  share_source="$BATS_TEST_TMPDIR/reference"
  mkdir -p "$share_source"
  jq -n \
    --arg source "$share_source" \
    '{schemaVersion: 1, shares: [
      {source: $source, mountPoint: "/mnt/reference", mode: {unexpected: true}}
    ]}' >"$REPO/codex-vm.json"

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --dry-run --json"

  [ "$status" -eq 2 ]
  [[ "$output" == *"mode must be ro or rw"* ]]
  [[ "$output" != *"Traceback"* ]]
  [ ! -e "$REPO/.codex-vm" ]
}

@test "start gives authorized read-only and read-write shares distinct host export slots" {
  make_fake_vm_runtime
  host_home="$BATS_TEST_TMPDIR/host-home"
  read_source="$BATS_TEST_TMPDIR/"$'reference\n'
  write_source="$BATS_TEST_TMPDIR/output"
  mkdir -p "$host_home/.config/ai-plugins" "$read_source" "$write_source"
  project_id="$(cd "$REPO" && "$ROOT/scripts/codex-vm/vm-codex" status --json | jq -r .project_id)"
  jq -n \
    --arg project_id "$project_id" \
    --arg read_source "$read_source" \
    --arg write_source "$write_source" \
    '{schemaVersion: 1, projects: {($project_id): [
      {source: $read_source, mountPoint: "/mnt/reference", mode: "ro"},
      {source: $write_source, mountPoint: "/mnt/output", mode: "rw"}
    ]}}' >"$host_home/.config/ai-plugins/codex-vm-shares.json"
  jq -n \
    --arg read_source "$read_source" \
    --arg write_source "$write_source" \
    '{schemaVersion: 1, shares: [
      {source: $read_source, mountPoint: "/mnt/reference", mode: "ro"},
      {source: $write_source, mountPoint: "/mnt/output", mode: "rw"}
    ]}' >"$REPO/codex-vm.json"

  run env HOME="$host_home" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"

  [ "$status" -eq 0 ]
  manifest="$REPO/.codex-vm/bootstrap/shares.json"
  [ "$(jq -r '.shares[] | select(.mountPoint == "/mnt/reference") | [.mode, .slot] | @tsv' "$manifest")" = $'ro\t0' ]
  [ "$(jq -r '.shares[] | select(.mountPoint == "/mnt/output") | [.mode, .slot] | @tsv' "$manifest")" = $'rw\t0' ]
  run "$CODEX_VM_PYTHON" - "$FAKE_BWRAP_ARGS_NUL" "$read_source" "$write_source" <<'PY'
import pathlib
import sys

arguments = pathlib.Path(sys.argv[1]).read_bytes().split(b"\0")
arguments = [argument.decode() for argument in arguments if argument]
expected = [
    ["--ro-bind", sys.argv[2], "/runtime/extra-shares/ro-0"],
    ["--bind", sys.argv[3], "/runtime/extra-shares/rw-0"],
]
for triple in expected:
    if not any(arguments[index:index + 3] == triple for index in range(len(arguments) - 2)):
        raise SystemExit(f"missing host export binding: {triple}")
PY
  if [ "$status" -ne 0 ]; then
    printf '%s\n' "$output" >&3
  fi
  [ "$status" -eq 0 ]

  run env HOME="$host_home" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' stop"
  [ "$status" -eq 0 ]
}

@test "start rejects extra-share changes while the VM is running" {
  make_fake_vm_runtime
  host_home="$BATS_TEST_TMPDIR/host-home"
  share_source="$BATS_TEST_TMPDIR/reference"
  mkdir -p "$host_home/.config/ai-plugins" "$share_source"

  run env HOME="$host_home" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"
  [ "$status" -eq 0 ]
  project_id="$(jq -r .project_id <<<"$output")"

  jq -n \
    --arg project_id "$project_id" \
    --arg source "$share_source" \
    '{schemaVersion: 1, projects: {($project_id): [
      {source: $source, mountPoint: "/mnt/reference", mode: "ro"}
    ]}}' >"$host_home/.config/ai-plugins/codex-vm-shares.json"
  jq -n \
    --arg source "$share_source" \
    '{schemaVersion: 1, shares: [
      {source: $source, mountPoint: "/mnt/reference", mode: "ro"}
    ]}' >"$REPO/codex-vm.json"

  run env HOME="$host_home" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"
  restart_status="$status"
  restart_output="$output"
  run env HOME="$host_home" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' stop"
  [ "$status" -eq 0 ]

  [ "$restart_status" -eq 2 ]
  [[ "$restart_output" == *"extra shares changed; run vm-codex stop before restarting"* ]]
}

@test "start rejects replacement of a shared directory while the VM is running" {
  make_fake_vm_runtime
  host_home="$BATS_TEST_TMPDIR/host-home"
  share_source="$BATS_TEST_TMPDIR/reference"
  original_source="$BATS_TEST_TMPDIR/original-reference"
  mkdir -p "$host_home/.config/ai-plugins" "$share_source"
  project_id="$(cd "$REPO" && "$ROOT/scripts/codex-vm/vm-codex" status --json | jq -r .project_id)"
  jq -n \
    --arg project_id "$project_id" \
    --arg source "$share_source" \
    '{schemaVersion: 1, projects: {($project_id): [
      {source: $source, mountPoint: "/mnt/reference", mode: "ro"}
    ]}}' >"$host_home/.config/ai-plugins/codex-vm-shares.json"
  jq -n \
    --arg source "$share_source" \
    '{schemaVersion: 1, shares: [
      {source: $source, mountPoint: "/mnt/reference", mode: "ro"}
    ]}' >"$REPO/codex-vm.json"

  run env HOME="$host_home" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"
  [ "$status" -eq 0 ]
  mv "$share_source" "$original_source"
  mkdir "$share_source"

  run env HOME="$host_home" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"
  restart_status="$status"
  restart_output="$output"
  run env HOME="$host_home" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' stop"
  [ "$status" -eq 0 ]

  [ "$restart_status" -eq 2 ]
  [[ "$restart_output" == *"extra shares changed; run vm-codex stop before restarting"* ]]
}

@test "start terminates a surviving exporter before recovering from a QEMU crash" {
  make_fake_vm_runtime

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"
  [ "$status" -eq 0 ]
  project_id="$(jq -r .project_id <<<"$output")"
  qemu_pid="$(jq -r .pid <<<"$output")"
  runtime_dir="$XDG_RUNTIME_DIR/ai-plugins-codex-vm/$project_id"
  read -r old_exporter_pid _ <"$runtime_dir/virtiofsd.pid"
  kill "$qemu_pid"
  for _ in $(seq 1 50); do
    kill -0 "$qemu_pid" 2>/dev/null || break
    sleep 0.02
  done

  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --json"
  restart_status="$status"
  old_exporter_alive=false
  if kill -0 "$old_exporter_pid" 2>/dev/null; then
    old_exporter_alive=true
    kill "$old_exporter_pid" 2>/dev/null || true
  fi
  run bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' stop"
  [ "$status" -eq 0 ]

  [ "$restart_status" -eq 0 ]
  [ "$old_exporter_alive" = false ]
}

@test "start rejects a writable share containing the owner authorization" {
  host_home="$BATS_TEST_TMPDIR/host-home"
  authorization_dir="$host_home/.config/ai-plugins"
  mkdir -p "$authorization_dir"
  project_id="$(cd "$REPO" && "$ROOT/scripts/codex-vm/vm-codex" status --json | jq -r .project_id)"
  jq -n \
    --arg project_id "$project_id" \
    --arg source "$authorization_dir" \
    '{schemaVersion: 1, projects: {($project_id): [
      {source: $source, mountPoint: "/mnt/owner-config", mode: "rw"}
    ]}}' >"$authorization_dir/codex-vm-shares.json"
  jq -n \
    --arg source "$authorization_dir" \
    '{schemaVersion: 1, shares: [
      {source: $source, mountPoint: "/mnt/owner-config", mode: "rw"}
    ]}' >"$REPO/codex-vm.json"

  run env HOME="$host_home" bash -c "cd '$REPO' && '$ROOT/scripts/codex-vm/vm-codex' start --dry-run --json"

  [ "$status" -eq 2 ]
  [[ "$output" == *"writable share must not contain the owner authorization"* ]]
  [ ! -e "$REPO/.codex-vm" ]
}
