# Toolchain and dependency boundaries

- Enter `nix develop`; `flake.nix` and `flake.lock` select the toolchain. Do not install global toolchains manually.
- Devshell redirects npm global installs/cache into ignored `.dependencies/`; Cargo state is also repository-local. Never commit dependency caches.
- Root `package.json`/`package-lock.json` pin Promptfoo and the Codex SDK for eval resolution. This is an intentional root npm project limited to eval infrastructure.
- Rust workspaces: `tiber/Cargo.toml`, `plugins/development-system/components/tiber/rust/Cargo.toml`; coordinator crate: `plugins/development-system/components/development-discipline/rust/Cargo.toml`.
- Bats exercises shell/plugin integrations; `just` is the command interface; Lefthook owns local commit checks and linked-worktree bootstrap.
- Linux x86_64 and Codex are required release targets. Other harnesses/architectures require separate verified support.
