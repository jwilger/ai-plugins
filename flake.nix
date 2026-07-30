{
  description = "ai-plugins — a multi-harness AI plugin marketplace (Claude Code, Codex, and others)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
  };

  outputs =
    { nixpkgs, ... }:
    let
      supportedSystems = [
        "aarch64-darwin"
        "aarch64-linux"
        "x86_64-darwin"
        "x86_64-linux"
      ];
    in
    {
      devShells = nixpkgs.lib.genAttrs supportedSystems (
        system:
        let
          pkgs = import nixpkgs { inherit system; };
          beadsRelease = {
            "x86_64-linux" = {
              asset = "beads_1.1.2_linux_amd64.tar.gz";
              hash = "sha256-py1x7TdJVdyfg6D5C1S9e2oAFnCd0Wdq4uNoZR7UAcI=";
            };
            "aarch64-linux" = {
              asset = "beads_1.1.2_linux_arm64.tar.gz";
              hash = "sha256-oTQBX69L4KQ/hoGo1gLq8LfCVclX8J08kzJXyMkv3RA=";
            };
            "x86_64-darwin" = {
              asset = "beads_1.1.2_darwin_amd64.tar.gz";
              hash = "sha256-DpTekxnJ1my34AOLsX669d0v5mnjZqS5FTUotHShqPY=";
            };
            "aarch64-darwin" = {
              asset = "beads_1.1.2_darwin_arm64.tar.gz";
              hash = "sha256-mwE3qDoq/TQ+Kr0qUGvnLqAychAA92Zpws+Bcp54UB0=";
            };
          }.${system};
          beads = pkgs.stdenvNoCC.mkDerivation {
            pname = "beads";
            version = "1.1.2";
            src = pkgs.fetchurl {
              url = "https://github.com/gastownhall/beads/releases/download/v1.1.2/${beadsRelease.asset}";
              hash = beadsRelease.hash;
            };
            sourceRoot = ".";
            unpackPhase = "tar -xzf $src";
            installPhase = ''
              mkdir -p "$out/bin"
              install -m 0755 bd "$out/bin/bd"
            '';
          };
          doltRelease = {
            "x86_64-linux" = {
              asset = "dolt-linux-amd64.tar.gz";
              hash = "sha256-/6+nzBcsraX3fKP7ljBlRd2sRKERYl91+HAwbH8ZcwE=";
            };
            "aarch64-linux" = {
              asset = "dolt-linux-arm64.tar.gz";
              hash = "sha256-Xo9NvmGTHDb4NZAi7jIzfl2vZaugail5GgZuUGd8izo=";
            };
            "x86_64-darwin" = {
              asset = "dolt-darwin-amd64.tar.gz";
              hash = "sha256-GieuFVQiAgIOflqKg3PglFVDqQGd+iFW0TVzooAbQIE=";
            };
            "aarch64-darwin" = {
              asset = "dolt-darwin-arm64.tar.gz";
              hash = "sha256-itrXSTUGHxNTkHhD5MGpJviO+W3J0jOCSjbNKSy3qj8=";
            };
          }.${system};
          dolt = pkgs.stdenvNoCC.mkDerivation {
            pname = "dolt";
            version = "2.2.3";
            src = pkgs.fetchurl {
              url = "https://github.com/dolthub/dolt/releases/download/v2.2.3/${doltRelease.asset}";
              hash = doltRelease.hash;
            };
            sourceRoot = ".";
            unpackPhase = "tar -xzf $src";
            installPhase = ''
              mkdir -p "$out/bin"
              install -m 0755 "$(find . -type f -path '*/bin/dolt' -print -quit)" "$out/bin/dolt"
            '';
          };
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
            ++ [ beads dolt ]
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

            # Put project-local installs first so locally installed tools win.
            export PATH="$CARGO_INSTALL_ROOT/bin:$DEPENDENCIES_DIR/npm/bin:$PATH"

            # EMC is local development tooling. GitHub Actions deliberately skips
            # this installation; CI must not realize or exercise EMC.
            if [ -z "''${CI:-}" ]; then
              emc_version="0.1.13"
              emc_bin="$CARGO_INSTALL_ROOT/bin/emc"

              if [ -x "$emc_bin" ] && cargo install --list | grep -Fqx "emc v$emc_version:"; then
                echo "EMC $emc_version is already installed."
              else
                echo "Installing EMC $emc_version."
                if ! cargo install --locked --force --version "$emc_version" emc; then
                  printf 'emc.install_failed version=%s: check network access and retry with cargo install --locked --force --version %s emc.\n' \
                    "$emc_version" "$emc_version" >&2
                  exit 1
                fi
              fi
            fi

            echo "ai-plugins devshell ready."
            echo "  just:  $(just --version) · node $(node --version) · npm $(npm --version)"
            echo "  Global npm installs -> ./.dependencies/ (git-ignored)"
          '';
          };
        }
      );
    };
}
