{
  description = "ai-plugins — a multi-harness AI plugin marketplace (Claude Code, Codex, and others)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    microvm = {
      url = "github:microvm-nix/microvm.nix";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, nixpkgs, microvm, ... }:
    let
      supportedSystems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
      codexVmPkgs = import nixpkgs { system = "x86_64-linux"; };
      codexVmQemuRuntimeArgs = codexVmPkgs.writeShellScript "codex-qemu-runtime-args" ''
        port="''${CODEX_VM_SSH_PORT:-}"
        case "$port" in
          ""|*[!0-9]*)
            echo "CODEX_VM_SSH_PORT must be a decimal port" >&2
            exit 2
            ;;
        esac
        if [ "$port" -lt 1024 ] || [ "$port" -gt 65535 ]; then
          echo "CODEX_VM_SSH_PORT must be between 1024 and 65535" >&2
          exit 2
        fi
        printf '%s\n' "-nic user,model=virtio-net-pci,hostfwd=tcp:127.0.0.1:$port-:22"
      '';
      codexVm = nixpkgs.lib.nixosSystem {
        system = "x86_64-linux";
        modules = [
          microvm.nixosModules.microvm
          (
            { config, pkgs, ... }:
            let
              authorizedKeysCommand = pkgs.writeShellScript "codex-authorized-keys" ''
                if [ "$1" = codex ] && [ -r /run/host-keys/codex.pub ]; then
                  exec ${pkgs.coreutils}/bin/cat /run/host-keys/codex.pub
                fi
              '';
              codexVmEnter = pkgs.writeScriptBin "codex-vm-enter" ''
                #!${pkgs.python3}/bin/python3
                import json
                import os
                import pathlib
                import sys

                environment_path = pathlib.Path("/run/codex-vm-environment/environment.json")
                try:
                    delta = json.loads(environment_path.read_text())
                except (OSError, json.JSONDecodeError) as error:
                    raise SystemExit(f"codex-vm-enter: environment is unavailable: {error}")
                if not isinstance(delta, dict):
                    raise SystemExit("codex-vm-enter: environment delta is not an object")
                environment = {
                    "HOME": "/home/codex",
                    "USER": "codex",
                    "LOGNAME": "codex",
                    "SHELL": "/bin/bash",
                    "PATH": "/run/current-system/sw/bin:/usr/bin:/bin",
                    "LANG": "C.UTF-8",
                    "TERM": "xterm-256color",
                }
                for key, value in delta.items():
                    if not isinstance(key, str) or not key or "=" in key or "\0" in key:
                        raise SystemExit("codex-vm-enter: invalid environment variable name")
                    if value is None:
                        environment.pop(key, None)
                    elif isinstance(value, str) and "\0" not in value:
                        environment[key] = value
                    else:
                        raise SystemExit(f"codex-vm-enter: invalid value for {key}")
                if len(sys.argv) < 2:
                    raise SystemExit("usage: codex-vm-enter shell | COMMAND [ARG ...]")
                os.chdir("/work")
                command = ["/bin/bash", "--noprofile", "--norc", "-i"] if sys.argv[1] == "shell" else sys.argv[1:]
                os.execvpe(command[0], command, environment)
              '';
              refreshEnvironment = pkgs.writeShellScript "codex-vm-refresh-environment" ''
                set -euo pipefail
                runtime=/run/codex-vm-environment
                tree="$runtime/tree"
                manifest=/run/codex-vm-bootstrap/envrc/manifest.json
                old_mount="$runtime/mounted-project"
                old_envrc="$runtime/mounted-envrc"
                ${pkgs.coreutils}/bin/install -d -m 0711 "$runtime"
                if [ -r "$old_envrc" ]; then
                  mounted_envrc="$(${pkgs.coreutils}/bin/cat "$old_envrc")"
                  case "$mounted_envrc" in "$tree"/*) ;; *) exit 2 ;; esac
                  ${pkgs.util-linux}/bin/mountpoint -q "$mounted_envrc" && ${pkgs.util-linux}/bin/umount "$mounted_envrc" || true
                fi
                if [ -r "$old_mount" ]; then
                  mounted_project="$(${pkgs.coreutils}/bin/cat "$old_mount")"
                  case "$mounted_project" in "$tree"/*) ;; *) exit 2 ;; esac
                  ${pkgs.util-linux}/bin/mountpoint -q "$mounted_project" && ${pkgs.util-linux}/bin/umount "$mounted_project" || true
                fi
                ${pkgs.coreutils}/bin/rm -rf "$tree"
                project_mirror="$(${pkgs.python3}/bin/python3 - "$manifest" "$tree" <<'PY'
                import hashlib
                import json
                import os
                import pathlib
                import sys

                manifest_path = pathlib.Path(sys.argv[1])
                tree = pathlib.Path(sys.argv[2])
                payload = json.loads(manifest_path.read_text())
                if set(payload) != {"files", "projectRelativePath", "projectRoot", "schemaVersion"} or payload["schemaVersion"] != 1:
                    raise SystemExit("invalid envrc manifest schema")
                project_relative = pathlib.PurePosixPath(payload["projectRelativePath"])
                if project_relative.is_absolute() or ".." in project_relative.parts or not project_relative.parts:
                    raise SystemExit("invalid envrc project path")
                project_mirror = tree.joinpath(*project_relative.parts)
                project_mirror.mkdir(mode=0o755, parents=True)
                for entry in payload["files"]:
                    if set(entry) != {"kind", "path", "relativePath", "sha256", "staged"}:
                        raise SystemExit("invalid envrc file entry")
                    relative = pathlib.PurePosixPath(entry["relativePath"])
                    if relative.is_absolute() or ".." in relative.parts or relative.name != ".envrc":
                        raise SystemExit("invalid envrc file path")
                    source = pathlib.Path("/run/codex-vm-bootstrap/envrc/files") / entry["staged"]
                    data = source.read_bytes()
                    if hashlib.sha256(data).hexdigest() != entry["sha256"]:
                        raise SystemExit("envrc snapshot digest mismatch")
                    if entry["kind"] == "ancestor":
                        target = tree.joinpath(*relative.parts)
                        target.parent.mkdir(mode=0o755, parents=True, exist_ok=True)
                        target.write_bytes(data)
                        os.chmod(target, 0o600)
                        os.chown(target, 1000, 1000)
                    elif entry["kind"] != "project":
                        raise SystemExit("invalid envrc file kind")
                print(project_mirror)
                PY
                )"
                ${pkgs.coreutils}/bin/chown -R codex:codex "$tree"
                ${pkgs.util-linux}/bin/mount --bind /work "$project_mirror"
                printf '%s\n' "$project_mirror" >"$old_mount"
                project_snapshot="$(${pkgs.jq}/bin/jq -er '.files[] | select(.kind == "project") | .staged' "$manifest" 2>/dev/null || true)"
                if [ -n "$project_snapshot" ]; then
                  staged_envrc="/run/codex-vm-bootstrap/envrc/files/$project_snapshot"
                  ${pkgs.coreutils}/bin/install -m 0600 -o codex -g codex "$staged_envrc" "$runtime/project.envrc"
                  ${pkgs.util-linux}/bin/mount --bind "$runtime/project.envrc" "$project_mirror/.envrc"
                  ${pkgs.util-linux}/bin/mount -o remount,bind,ro "$project_mirror/.envrc"
                  printf '%s\n' "$project_mirror/.envrc" >"$old_envrc"
                else
                  ${pkgs.coreutils}/bin/rm -f "$old_envrc"
                fi
                ${pkgs.coreutils}/bin/install -d -m 0700 -o codex -g codex "$runtime/config" "$runtime/data"
                printf '%s\n' 'source_up_if_present() { source_up_if_exists "$@"; }' >"$runtime/config/direnvrc"
                ${pkgs.coreutils}/bin/chown codex:codex "$runtime/config/direnvrc"
                ${pkgs.coreutils}/bin/chmod 0600 "$runtime/config/direnvrc"
                delta="$runtime/.environment.json"
                envrc_count="$(${pkgs.jq}/bin/jq -er '.files | length' "$manifest")"
                if [ "$envrc_count" -gt 0 ]; then
                  if ! ${pkgs.util-linux}/bin/runuser -u codex -- \
                    ${pkgs.coreutils}/bin/env -i \
                      HOME=/home/codex USER=codex LOGNAME=codex SHELL=/bin/bash \
                      PATH=/run/current-system/sw/bin:/usr/bin:/bin LANG=C.UTF-8 TERM=xterm-256color \
                      DIRENV_CONFIG="$runtime/config" XDG_DATA_HOME="$runtime/data" DIRENV_LOG_FORMAT= \
                      ${pkgs.bash}/bin/bash --noprofile --norc -c \
                        'cd "$1" && direnv allow . >/dev/null && exec direnv export json' _ "$project_mirror" \
                    >"$delta" 2>/dev/null; then
                    ${pkgs.coreutils}/bin/rm -f "$delta"
                    echo "project .envrc evaluation failed inside the guest" >&2
                    exit 1
                  fi
                else
                  printf '{}\n' >"$delta"
                fi
                ${pkgs.python3}/bin/python3 - "$delta" "$runtime/environment.json" "$project_mirror" <<'PY'
                import json
                import os
                import pathlib
                import sys

                source = pathlib.Path(sys.argv[1])
                destination = pathlib.Path(sys.argv[2])
                mirror = sys.argv[3]
                delta = json.loads(source.read_text())
                if not isinstance(delta, dict):
                    raise SystemExit("direnv did not return an environment object")
                baseline = {
                    "HOME": "/home/codex", "USER": "codex", "LOGNAME": "codex",
                    "SHELL": "/bin/bash", "PATH": "/run/current-system/sw/bin:/usr/bin:/bin",
                    "LANG": "C.UTF-8", "TERM": "xterm-256color",
                }
                prohibited_names = {"SSH_AUTH_SOCK", "DOCKER_HOST", "CONTAINER_HOST", "NIX_REMOTE"}
                prohibited_fragments = (
                    "/run/host-keys", "/run/codex-vm-bootstrap", "/.codex-vm",
                    "/dev/kvm", "/dev/vhost", "/run/docker.sock", "/var/run/docker.sock",
                )
                cleaned = {}
                for key, value in delta.items():
                    if key.startswith("DIRENV_") or key in {"PWD", "OLDPWD", "SHLVL", "_"}:
                        continue
                    if key in prohibited_names:
                        raise SystemExit(f"prohibited host-boundary variable: {key}")
                    if value is not None and not isinstance(value, str):
                        raise SystemExit(f"invalid environment value for {key}")
                    if isinstance(value, str):
                        value = value.replace(mirror, "/work")
                        if any(fragment in value for fragment in prohibited_fragments):
                            raise SystemExit(f"prohibited host-boundary value for {key}")
                    if value != baseline.get(key):
                        cleaned[key] = value
                temporary = destination.with_name(".environment.json.tmp")
                temporary.write_text(json.dumps(cleaned, separators=(",", ":"), sort_keys=True) + "\n")
                os.chmod(temporary, 0o600)
                os.chown(temporary, 1000, 1000)
                os.replace(temporary, destination)
                source.unlink(missing_ok=True)
                PY
              '';
              mountShares = pkgs.writeShellScript "codex-vm-mount-shares" ''
                set -euo pipefail
                runtime=/run/codex-vm-shares
                manifest=/run/codex-vm-bootstrap/shares.json
                ${pkgs.coreutils}/bin/install -d -m 0700 "$runtime"
                live_targets="$runtime/live-targets"
                ${pkgs.python3}/bin/python3 - >"$live_targets" <<'PY'
                import re

                pattern = re.compile(r"/mnt/[A-Za-z0-9][A-Za-z0-9._-]*\Z")
                targets = set()
                with open("/proc/self/mountinfo", encoding="utf-8") as mountinfo:
                    for line in mountinfo:
                        target = line.split(" - ", 1)[0].split()[4]
                        if pattern.fullmatch(target):
                            targets.add(target)
                print("\n".join(sorted(targets)))
                PY
                while IFS= read -r target; do
                  while ${pkgs.util-linux}/bin/mountpoint -q "$target"; do
                    ${pkgs.util-linux}/bin/umount "$target"
                  done
                done <"$live_targets"
                ${pkgs.coreutils}/bin/rm -f "$live_targets"
                declarations="$runtime/declarations"
                ${pkgs.python3}/bin/python3 - "$manifest" >"$declarations" <<'PY'
                import base64
                import json
                import pathlib
                import re
                import sys

                pattern = re.compile(r"/mnt/[A-Za-z0-9][A-Za-z0-9._-]*\Z")
                payload = json.loads(pathlib.Path(sys.argv[1]).read_text())
                if not isinstance(payload, dict) or set(payload) != {"schemaVersion", "shares"} or payload["schemaVersion"] != 1:
                    raise SystemExit("invalid share manifest schema")
                shares = payload["shares"]
                if not isinstance(shares, list):
                    raise SystemExit("share manifest entries must be an array")
                seen = set()
                counts = {"ro": 0, "rw": 0}
                for share in shares:
                    if not isinstance(share, dict) or set(share) != {"mode", "mountPoint", "slot"}:
                        raise SystemExit("invalid share manifest entry")
                    mode = share["mode"]
                    target = share["mountPoint"]
                    slot = share["slot"]
                    if mode not in counts or type(slot) is not int or slot != counts[mode] or slot > 3:
                        raise SystemExit("invalid share slot assignment")
                    if not isinstance(target, str) or not pattern.fullmatch(target) or target in seen:
                        raise SystemExit("invalid share mount point")
                    counts[mode] += 1
                    seen.add(target)
                    encoded = base64.b64encode(target.encode()).decode()
                    print(f"{mode} {slot} {encoded}")
                PY
                while read -r mode slot encoded; do
                  [ -n "$mode" ] || continue
                  target="$(${pkgs.coreutils}/bin/printf '%s' "$encoded" | ${pkgs.coreutils}/bin/base64 --decode)"
                  source="/run/codex-vm-extra/$mode/$slot"
                  ${pkgs.coreutils}/bin/install -d -m 0755 "$target"
                  ${pkgs.util-linux}/bin/mount --bind "$source" "$target"
                  if [ "$mode" = ro ]; then
                    ${pkgs.util-linux}/bin/mount -o remount,bind,ro "$target"
                  fi
                done <"$declarations"
                ${pkgs.coreutils}/bin/rm -f "$declarations"
              '';
            in
            {
              networking.hostName = "codex-vm";
              system.stateVersion = "25.11";

              users.groups.codex.gid = 1000;
              users.users.codex = {
                isNormalUser = true;
                uid = 1000;
                group = "codex";
                home = "/home/codex";
                createHome = true;
                extraGroups = [ "wheel" ];
              };

              security.sudo.wheelNeedsPassword = false;
              networking.useDHCP = true;
              networking.nftables = {
                enable = true;
                tables.codex-vm-egress = {
                  family = "inet";
                  content = ''
                    chain output {
                      type filter hook output priority -5; policy accept;
                      oifname "lo" accept
                      ct state established,related accept
                      ip daddr 10.0.2.2 udp sport 68 udp dport 67 accept
                      ip daddr 10.0.2.3 udp dport 53 accept
                      ip daddr 10.0.2.3 tcp dport 53 accept
                      ip6 daddr fec0::3 udp dport 53 accept
                      ip6 daddr fec0::3 tcp dport 53 accept
                      ip daddr { 10.0.0.0/8, 172.16.0.0/12, 192.168.0.0/16, 169.254.0.0/16 } reject
                      ip6 daddr { fc00::/7, fe80::/10 } reject
                    }
                  '';
                };
              };

              services.openssh = {
                enable = true;
                openFirewall = true;
                hostKeys = [
                  {
                    path = "/run/host-keys/ssh_host_ed25519_key";
                    type = "ed25519";
                  }
                ];
                settings = {
                  PasswordAuthentication = false;
                  KbdInteractiveAuthentication = false;
                  PermitRootLogin = "no";
                  AuthorizedKeysCommand = "${authorizedKeysCommand} %u";
                  AuthorizedKeysCommandUser = "nobody";
                };
              };

              systemd.services.codex-vm-bootstrap = {
                description = "Initialize project-local Codex preferences";
                wantedBy = [ "multi-user.target" ];
                before = [ "sshd.service" ];
                unitConfig.RequiresMountsFor = [
                  "/home/codex"
                  "/run/codex-vm-bootstrap"
                ];
                serviceConfig.Type = "oneshot";
                script = ''
                  codex_home=/home/codex/.codex
                  marker="$codex_home/.vm-preferences-initialized"
                  ${pkgs.coreutils}/bin/install -d -m 0700 -o codex -g codex "$codex_home"
                  if [ ! -e "$marker" ]; then
                    ${pkgs.coreutils}/bin/install -m 0600 -o codex -g codex \
                      /run/codex-vm-bootstrap/preferences.toml \
                      "$codex_home/config.toml"
                    ${pkgs.coreutils}/bin/touch "$marker"
                    ${pkgs.coreutils}/bin/chown codex:codex "$marker"
                    ${pkgs.coreutils}/bin/chmod 0600 "$marker"
                  fi
                '';
              };

              systemd.services.codex-vm-environment = {
                description = "Evaluate project environment inside the Codex VM";
                requires = [ "codex-vm-shares.service" ];
                after = [ "codex-vm-shares.service" ];
                unitConfig.RequiresMountsFor = [
                  "/work"
                  "/home/codex"
                  "/run/codex-vm-bootstrap"
                ];
                serviceConfig = {
                  Type = "oneshot";
                  ExecStart = refreshEnvironment;
                  UMask = "0077";
                };
              };

              systemd.services.codex-vm-shares = {
                description = "Mount explicitly authorized Codex VM shares";
                unitConfig.RequiresMountsFor = [
                  "/run/codex-vm-bootstrap"
                  "/run/codex-vm-extra/ro/0"
                  "/run/codex-vm-extra/ro/1"
                  "/run/codex-vm-extra/ro/2"
                  "/run/codex-vm-extra/ro/3"
                  "/run/codex-vm-extra/rw/0"
                  "/run/codex-vm-extra/rw/1"
                  "/run/codex-vm-extra/rw/2"
                  "/run/codex-vm-extra/rw/3"
                ];
                serviceConfig = {
                  Type = "oneshot";
                  ExecStart = mountShares;
                  UMask = "0077";
                };
              };

              environment.systemPackages = with pkgs; [
                bash
                cacert
                codex
                curl
                direnv
                git
                gh
                jq
                nix
                openssh
                ripgrep
                codexVmEnter
              ];

              nix.settings.experimental-features = [
                "nix-command"
                "flakes"
              ];

              microvm = {
                hypervisor = "qemu";
                vcpu = 4;
                mem = 8192;
                writableStoreOverlay = "/nix/.rw-store";
                extraArgsScript = "${codexVmQemuRuntimeArgs}";
                shares = [
                  {
                    proto = "virtiofs";
                    tag = "work";
                    source = "work-export";
                    mountPoint = "/work";
                  }
                  {
                    proto = "virtiofs";
                    tag = "host-keys";
                    source = "ssh-keys";
                    mountPoint = "/run/host-keys";
                    readOnly = true;
                  }
                  {
                    proto = "virtiofs";
                    tag = "bootstrap";
                    source = "bootstrap";
                    mountPoint = "/run/codex-vm-bootstrap";
                    readOnly = true;
                  }
                ] ++ builtins.concatLists (builtins.genList (index: [
                  {
                    proto = "virtiofs";
                    tag = "extra-ro-${toString index}";
                    source = "extra-shares/ro-${toString index}";
                    mountPoint = "/run/codex-vm-extra/ro/${toString index}";
                    readOnly = true;
                  }
                  {
                    proto = "virtiofs";
                    tag = "extra-rw-${toString index}";
                    source = "extra-shares/rw-${toString index}";
                    mountPoint = "/run/codex-vm-extra/rw/${toString index}";
                    readOnly = false;
                  }
                ]) 4);
                volumes = [
                  {
                    image = "nix-store-overlay.img";
                    mountPoint = config.microvm.writableStoreOverlay;
                    size = 65536;
                  }
                  {
                    image = "codex-home.img";
                    mountPoint = "/home/codex";
                    size = 32768;
                  }
                ];
              };
            }
          )
        ];
      };
      codexVmEnterPackage = builtins.head (
        builtins.filter (package: package.name == "codex-vm-enter") codexVm.config.environment.systemPackages
      );
      codexVmRefreshEnvironment = codexVm.config.systemd.services.codex-vm-environment.serviceConfig.ExecStart;
    in
    {
      nixosConfigurations.codex-vm = codexVm;

      checks.x86_64-linux.codex-vm-environment = codexVmPkgs.testers.runNixOSTest {
        name = "codex-vm-environment";
        nodes.machine =
          { pkgs, ... }:
          {
            system.stateVersion = "25.11";
            users.groups.codex.gid = 1000;
            users.users.codex = {
              isNormalUser = true;
              uid = 1000;
              group = "codex";
              home = "/home/codex";
              createHome = true;
            };
            environment.systemPackages = [
              codexVmEnterPackage
              pkgs.direnv
            ];
            systemd.services.codex-vm-environment = {
              serviceConfig = {
                Type = "oneshot";
                ExecStart = codexVmRefreshEnvironment;
                UMask = "0077";
              };
            };
          };
        testScript = ''
          import base64
          import hashlib
          import json
          import shlex

          machine.start()
          machine.wait_for_unit("multi-user.target")

          def put(path, content, mode="0600"):
              encoded = base64.b64encode(content.encode()).decode()
              parent = path.rsplit("/", 1)[0]
              machine.succeed(
                  "install -d -m 0700 " + shlex.quote(parent) +
                  "; printf %s " + shlex.quote(encoded) +
                  " | base64 -d >" + shlex.quote(path) +
                  "; chmod " + mode + " " + shlex.quote(path)
              )

          ancestor = "export PARENT_VALUE=parent\n"
          project = (
              "source_up_if_exists\n"
              "export PROJECT_VALUE=project\n"
              "export PROJECT_PATH=\"$PWD/bin\"\n"
              "unset HOME\n"
          )
          entries = []
          for kind, path, relative, content in [
              ("ancestor", "/workspace/.envrc", "workspace/.envrc", ancestor),
              ("project", "/workspace/project/.envrc", "workspace/project/.envrc", project),
          ]:
              digest = hashlib.sha256(content.encode()).hexdigest()
              staged = digest + ".envrc"
              put("/run/codex-vm-bootstrap/envrc/files/" + staged, content)
              entries.append({
                  "kind": kind,
                  "path": path,
                  "relativePath": relative,
                  "sha256": digest,
                  "staged": staged,
              })
          manifest = {
              "schemaVersion": 1,
              "projectRoot": "/workspace/project",
              "projectRelativePath": "workspace/project",
              "files": entries,
          }
          put("/run/codex-vm-bootstrap/envrc/manifest.json", json.dumps(manifest))
          put("/work/.envrc", project)
          machine.succeed("chown -R codex:codex /work; chmod 0755 /work")

          machine.succeed("systemctl start codex-vm-environment.service")
          output = machine.succeed(
              "runuser -u codex -- env AMBIENT_GUEST=must-not-leak codex-vm-enter env"
          )
          exported = dict(line.split("=", 1) for line in output.splitlines() if "=" in line)
          assert exported["PARENT_VALUE"] == "parent", exported
          assert exported["PROJECT_VALUE"] == "project", exported
          assert exported["PROJECT_PATH"] == "/work/bin", exported
          assert "HOME" not in exported, exported
          assert "AMBIENT_GUEST" not in exported, exported

          prohibited = "export SSH_AUTH_SOCK=/run/user/1000/agent\n"
          digest = hashlib.sha256(prohibited.encode()).hexdigest()
          staged = digest + ".envrc"
          put("/run/codex-vm-bootstrap/envrc/files/" + staged, prohibited)
          manifest["files"][-1].update({"sha256": digest, "staged": staged})
          put("/run/codex-vm-bootstrap/envrc/manifest.json", json.dumps(manifest))
          machine.fail("systemctl start codex-vm-environment.service")
          output = machine.succeed("runuser -u codex -- codex-vm-enter env")
          exported = dict(line.split("=", 1) for line in output.splitlines() if "=" in line)
          assert exported["PROJECT_VALUE"] == "project", exported
          assert "SSH_AUTH_SOCK" not in exported, exported
        '';
      };

      checks.x86_64-linux.codex-vm-shares = codexVmPkgs.testers.runNixOSTest {
        name = "codex-vm-shares";
        nodes.machine = {
          system.stateVersion = "25.11";
          systemd.services.codex-vm-shares = {
            serviceConfig = {
              Type = "oneshot";
              ExecStart = codexVm.config.systemd.services.codex-vm-shares.serviceConfig.ExecStart;
              UMask = "0077";
            };
          };
        };
        testScript = ''
          import base64
          import json
          import shlex

          machine.start()
          machine.wait_for_unit("multi-user.target")

          def put(path, content, mode="0600"):
              encoded = base64.b64encode(content.encode()).decode()
              parent = path.rsplit("/", 1)[0]
              machine.succeed(
                  "install -d -m 0700 " + shlex.quote(parent) +
                  "; printf %s " + shlex.quote(encoded) +
                  " | base64 -d >" + shlex.quote(path) +
                  "; chmod " + mode + " " + shlex.quote(path)
              )

          machine.succeed(
              "install -d /run/codex-vm-extra/ro/0 /run/codex-vm-extra/rw/0"
          )
          put("/run/codex-vm-extra/ro/0/reference.txt", "reference\n", "0644")
          manifest = {
              "schemaVersion": 1,
              "shares": [
                  {"mountPoint": "/mnt/reference", "mode": "ro", "slot": 0},
                  {"mountPoint": "/mnt/output", "mode": "rw", "slot": 0},
              ],
          }
          put("/run/codex-vm-bootstrap/shares.json", json.dumps(manifest))

          machine.succeed("systemctl start codex-vm-shares.service")
          machine.succeed("grep -Fx reference /mnt/reference/reference.txt")
          machine.fail("touch /mnt/reference/root-must-not-write")
          machine.succeed("touch /mnt/output/guest-can-write")

          machine.succeed(
              "systemd-run --unit=hold-initial-share --service-type=exec "
              "sh -c 'cd /mnt/reference && exec /run/current-system/sw/bin/sleep infinity'"
          )
          machine.wait_for_unit("hold-initial-share.service")
          manifest["shares"] = []
          put("/run/codex-vm-bootstrap/shares.json", json.dumps(manifest))
          machine.fail("systemctl start codex-vm-shares.service")
          machine.succeed("mountpoint -q /mnt/reference")
          machine.fail("mountpoint -q /mnt/output")
          machine.succeed("systemctl stop hold-initial-share.service")
          machine.succeed("systemctl start codex-vm-shares.service")
          machine.fail("mountpoint -q /mnt/reference")
          machine.fail("mountpoint -q /mnt/output")

          machine.succeed(
              "rm -f /run/codex-vm-extra/rw/0/guest-can-write; "
              "rmdir /run/codex-vm-extra/rw/0"
          )
          manifest["shares"] = [
              {"mountPoint": "/mnt/reference", "mode": "ro", "slot": 0},
              {"mountPoint": "/mnt/output", "mode": "rw", "slot": 0},
          ]
          put("/run/codex-vm-bootstrap/shares.json", json.dumps(manifest))
          machine.fail("systemctl start codex-vm-shares.service")
          machine.succeed("mountpoint -q /mnt/reference")
          machine.fail("mountpoint -q /mnt/output")
          machine.succeed("rm -f /run/codex-vm-shares/mounted.json")

          machine.succeed(
              "systemd-run --unit=hold-share --service-type=exec "
              "sh -c 'cd /mnt/reference && exec /run/current-system/sw/bin/sleep infinity'"
          )
          machine.wait_for_unit("hold-share.service")
          machine.succeed("install -d /run/codex-vm-extra/rw/0")
          machine.fail("systemctl start codex-vm-shares.service")
          machine.succeed("mountpoint -q /mnt/reference")
          machine.succeed("systemctl stop hold-share.service")
          machine.succeed("systemctl start codex-vm-shares.service")
          machine.succeed("mountpoint -q /mnt/reference")
          machine.succeed("mountpoint -q /mnt/output")

          manifest["shares"] = []
          put("/run/codex-vm-bootstrap/shares.json", json.dumps(manifest))
          machine.succeed("systemctl start codex-vm-shares.service")
          machine.fail("mountpoint -q /mnt/reference")
          machine.fail("mountpoint -q /mnt/output")
        '';
      };

      packages = nixpkgs.lib.genAttrs supportedSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          # cargo-zigbuild needs an SDK when linking Darwin targets from Linux.
          # Expose the pinned, platform-independent payload without attempting
          # to build Nixpkgs' Darwin-only apple-sdk wrapper derivation.
          apple-sdk-source = pkgs.apple-sdk_15.src;
        }
        // pkgs.lib.optionalAttrs (system == "x86_64-linux") {
          codex-vm-runner = codexVm.config.microvm.declaredRunner;
          codex-vm-qemu-runtime-args = codexVmQemuRuntimeArgs;

          vm-codex = pkgs.writeShellApplication {
            name = "vm-codex";
            runtimeInputs = with pkgs; [
              coreutils
              git
              jq
              procps
            ];
            runtimeEnv = {
              CODEX_VM_BWRAP = "${pkgs.bubblewrap}/bin/bwrap";
              CODEX_VM_FLOCK = "${pkgs.util-linux}/bin/flock";
              CODEX_VM_PYTHON = "${pkgs.python3}/bin/python3";
              CODEX_VM_RUNNER = "${codexVm.config.microvm.declaredRunner}/bin/microvm-run";
              CODEX_VM_SSH = "${pkgs.openssh}/bin/ssh";
              CODEX_VM_SSH_KEYGEN = "${pkgs.openssh}/bin/ssh-keygen";
              CODEX_VM_VIRTIOFSD_RUN = "${codexVm.config.microvm.declaredRunner}/bin/virtiofsd-run";
            };
            text = builtins.readFile ./scripts/codex-vm/vm-codex;
          };

          vm-shell = pkgs.writeShellApplication {
            name = "vm-shell";
            text = ''
              exec ${self.packages.${system}.vm-codex}/bin/vm-codex shell "$@"
            '';
          };

          codex-vm-tools = pkgs.symlinkJoin {
            name = "codex-vm-tools";
            paths = [
              self.packages.${system}.vm-codex
              self.packages.${system}.vm-shell
            ];
          };
        }
      );

      devShells = nixpkgs.lib.genAttrs supportedSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
        in
        {
          default = pkgs.mkShell {
          name = "ai-plugins";

          # Toolchain provided by Nix. Anything installed globally outside Nix
          # (npm -g, etc.) is redirected into ./.dependencies/ by the shellHook
          # below so it never leaks into your home directory.
          packages =
            (with pkgs; [
              bash
              git
              openssh
              jq
              ripgrep
              fd
              nodejs_22
              python312
              nix
              cargo
              cargo-mutants
              cargo-zigbuild
              file
              chromium
              clippy
              rustc
              rustfmt
              rustup
              zig
              just
              lefthook
              util-linux
              prettier
              bats
              actionlint
              yq-go
            ])
            ++ pkgs.lib.optionals pkgs.stdenv.isLinux [
              pkgs.bubblewrap
              pkgs.systemd
            ];

          shellHook = ''
            ${pkgs.lib.optionalString pkgs.stdenv.isLinux ''
              # Candidate verifiers resolve these exact flake-selected tools;
              # they must not discover security boundaries through caller PATH.
              export AI_PLUGINS_BWRAP_BIN="${pkgs.bubblewrap}/bin/bwrap"
              export AI_PLUGINS_PRLIMIT_BIN="${pkgs.util-linux}/bin/prlimit"
              export AI_PLUGINS_SYSTEMD_RUN_BIN="${pkgs.systemd}/bin/systemd-run"
              export AI_PLUGINS_SYSTEMCTL_BIN="${pkgs.systemd}/bin/systemctl"
            ''}

            # Give hook installation an unambiguous, lockfile-selected Lefthook
            # source and expected version.
            export AI_PLUGINS_LEFTHOOK_BIN="${pkgs.lefthook}/bin/lefthook"
            export AI_PLUGINS_LEFTHOOK_STORE_PATH="${pkgs.lefthook}"
            export AI_PLUGINS_LEFTHOOK_VERSION="${pkgs.lefthook.version}"

            # --- Project-local "global" dependency sandbox ---------------------
            # Everything a package manager would normally drop into $HOME instead
            # lands in ./.dependencies/ (git-ignored). Blow it away any time with
            # `rm -rf .dependencies` to get a clean slate.
            export DEPENDENCIES_DIR="$PWD/.dependencies"
            mkdir -p \
              "$DEPENDENCIES_DIR/npm/bin" \
              "$DEPENDENCIES_DIR/npm-cache" \
              "$DEPENDENCIES_DIR/cargo"

            # npm / node — `npm install -g <pkg>` installs here, bins on PATH.
            export NPM_CONFIG_PREFIX="$DEPENDENCIES_DIR/npm"
            export NPM_CONFIG_CACHE="$DEPENDENCIES_DIR/npm-cache"
            export NPM_CONFIG_USERCONFIG="$DEPENDENCIES_DIR/npmrc"

            # Cargo — project-local installs and registry state stay out of $HOME.
            export CARGO_HOME="$DEPENDENCIES_DIR/cargo"
            export CARGO_INSTALL_ROOT="$CARGO_HOME"

            export PATH="$CARGO_INSTALL_ROOT/bin:$DEPENDENCIES_DIR/npm/bin:$PATH"

            echo "ai-plugins devshell ready."
            echo "  just:  $(just --version) · node $(node --version) · npm $(npm --version)"
            echo "  Global npm installs -> ./.dependencies/ (git-ignored)"
          '';
          };
        }
      );
    };
}
