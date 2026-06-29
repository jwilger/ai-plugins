{
  description = "ai-plugins — a multi-harness AI plugin marketplace (Claude Code, and eventually Codex and others)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";
  };

  outputs =
    { self, nixpkgs, flake-utils }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        pkgs = import nixpkgs { inherit system; };
      in
      {
        devShells.default = pkgs.mkShell {
          name = "ai-plugins";

          # Toolchain provided by Nix. Anything installed *globally* outside Nix
          # (npm -g, cargo install, etc.) is redirected into ./.dependencies/
          # by the shellHook below so it never leaks into your home directory.
          packages = with pkgs; [
            # Core
            git
            jq            # validate / manipulate marketplace + plugin manifests
            ripgrep
            fd

            # Node — most MCP servers, hooks, and plugin tooling run on Node.
            nodejs_22

            # Rust — for plugins / tooling distributed as cargo crates.
            cargo
            rustc

            # JSON schema / formatting helpers handy for authoring plugins.
            prettier
          ];

          shellHook = ''
            # --- Project-local "global" dependency sandbox ---------------------
            # Everything a package manager would normally drop into $HOME instead
            # lands in ./.dependencies/ (git-ignored). Blow it away any time with
            # `rm -rf .dependencies` to get a clean slate.
            export DEPENDENCIES_DIR="$PWD/.dependencies"
            mkdir -p \
              "$DEPENDENCIES_DIR/npm/bin" \
              "$DEPENDENCIES_DIR/npm-cache" \
              "$DEPENDENCIES_DIR/cargo/bin"

            # npm / node — `npm install -g <pkg>` installs here, bins on PATH.
            export NPM_CONFIG_PREFIX="$DEPENDENCIES_DIR/npm"
            export NPM_CONFIG_CACHE="$DEPENDENCIES_DIR/npm-cache"
            export NPM_CONFIG_USERCONFIG="$DEPENDENCIES_DIR/npmrc"

            # cargo — `cargo install <crate>` installs here, bins on PATH.
            export CARGO_HOME="$DEPENDENCIES_DIR/cargo"

            # Put the project-local bin dirs first so locally installed tools win.
            export PATH="$DEPENDENCIES_DIR/npm/bin:$DEPENDENCIES_DIR/cargo/bin:$PATH"

            echo "ai-plugins devshell ready."
            echo "  Nix-provided: $(node --version), npm $(npm --version), cargo $(cargo --version | cut -d' ' -f2)"
            echo "  Global installs (npm -g / cargo install) -> ./.dependencies/ (git-ignored)"
          '';
        };
      }
    );
}
