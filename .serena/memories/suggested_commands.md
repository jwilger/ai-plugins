# Canonical commands

- Devshell: `nix develop`; execute commands with `nix develop -c ...`.
- Fast commit-owned gate: `nix develop -c just pre-commit`.
- Full CI-equivalent gate: `nix develop -c just ci`.
- Coordinator gates: `nix develop -c just development-discipline-rust`; Bats: `nix develop -c just bats`.
- Install shared worktree hooks from the primary checkout: `nix develop -c just worktree-hooks`; refresh after hook behavior or flake runtime changes.
- Remove owned linked worktrees through `nix develop -c just worktree-teardown .worktrees/<name>` after preserving work.
- Bump plugin and marketplace together: `scripts/bump-plugin-version.sh development-system patch|minor|major`.
- Eval wiring only: `nix develop -c scripts/evals/run.sh --dry-run`. Applicable local provider run: `nix develop -c scripts/evals/run.sh`; choose the smallest relevant suite/cases.
- `just evals` also shares results externally. Use the lower-level runner for local-only artifacts.
