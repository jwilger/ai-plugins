{
  description = "ai-plugins — a multi-harness AI plugin marketplace plus the sidequest control-plane crate (Claude Code, Codex, and others)";

  inputs = {
    nixpkgs.url = "github:NixOS/nixpkgs/nixos-unstable";
    flake-utils.url = "github:numtide/flake-utils";

    # Nightly Rust toolchain, version selected by ./rust-toolchain.toml and
    # pinned bit-for-bit by this input's revision in flake.lock.
    rust-overlay = {
      url = "github:oxalica/rust-overlay";
      inputs.nixpkgs.follows = "nixpkgs";
    };
  };

  outputs =
    { self, nixpkgs, flake-utils, rust-overlay }:
    flake-utils.lib.eachDefaultSystem (
      system:
      let
        overlays = [ (import rust-overlay) ];
        pkgs = import nixpkgs { inherit system overlays; };

        # The toolchain file is authoritative; the overlay revision pins the
        # exact nightly. (Package/release builds add crane on top of this.)
        rustToolchain = pkgs.rust-bin.fromRustupToolchainFile ./rust-toolchain.toml;
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

            # Rust toolchain (nightly, from rust-toolchain.toml) + command runner.
            rustToolchain
            just

            # Rust quality gates (reproducible via nixpkgs + flake.lock).
            cargo-nextest
            cargo-mutants
            cargo-audit
            release-plz

            # JSON schema / formatting helpers handy for authoring plugins.
            prettier

            # Shell / plugin-script tests.
            bats
          ];

          # Let rust-analyzer find the standard library sources.
          RUST_SRC_PATH = "${rustToolchain}/lib/rustlib/src/rust/library";

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

            # cargo — registry cache + `cargo install <crate>` land here.
            export CARGO_HOME="$DEPENDENCIES_DIR/cargo"

            # Put the project-local bin dirs first so locally installed tools win.
            export PATH="$DEPENDENCIES_DIR/npm/bin:$DEPENDENCIES_DIR/cargo/bin:$PATH"

            echo "ai-plugins devshell ready."
            echo "  rust:  $(rustc --version)"
            echo "  just:  $(just --version) · node $(node --version) · npm $(npm --version)"
            echo "  Global installs (npm -g / cargo install) -> ./.dependencies/ (git-ignored)"
          '';
        };
      }
    );
}
