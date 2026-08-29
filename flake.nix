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
              };

              services.openssh = {
                enable = true;
                settings = {
                  PasswordAuthentication = false;
                  KbdInteractiveAuthentication = false;
                  PermitRootLogin = "no";
                  AuthorizedKeysCommand = "${authorizedKeysCommand} %u";
                  AuthorizedKeysCommandUser = "nobody";
                };
              };

              environment.systemPackages = with pkgs; [
                bash
                git
                jq
                openssh
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
                ];
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
    in
    {
      nixosConfigurations.codex-vm = codexVm;

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
