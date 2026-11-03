# Contributing

Use `nix develop` for the pinned toolchain, then `just worktree-hooks` from the
primary checkout. Read [AGENTS.md](AGENTS.md), [engineering rules](docs/rules/),
and [architectural decisions](docs/adr/) before substantive changes. Propose
larger changes in an issue; keep patches focused and describe their validation.

Use Conventional Commits without Co-Authored-By trailers. Keep npm tooling in
the ignored `.dependencies/` directory. Bump changed plugins using
`scripts/bump-plugin-version.sh PLUGIN patch` (or minor/major as appropriate).

Run `just pre-commit`, `just validate-marketplace`, and `just ci` before claiming
completion. Follow the [evaluation guide](docs/evaluations.md) when model-facing
behavior changes; dry-run wiring is not behavior evidence. Lower-level eval
commands keep results local; `just evals` uploads results through Promptfoo share.
Contributions are governed by the repository's [AGPL license](LICENSE).
