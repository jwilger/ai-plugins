---
name: setup
description: Use when initializing or reconfiguring a repository with the development-system plugin, including delivery mode, worktrees, Beads with Dolt, workflow formulas, optional capabilities, and conflict checks.
---

# Development system setup

Setup is an explicit repository mutation and must run from the primary Git
checkout. Never require or permit a linked worktree for this operation.

Use the plugin-owned command:

```shell
<plugin-root>/bin/development-system setup --project <repo> --preset personal-trunk --dry-run
```

Show the preview and ask for explicit approval before replacing `--dry-run`
with `--apply --yes`. Ask customization questions one at a time.

The default is `--delivery direct-to-trunk` with `worktrees` and `beads`
enabled. Select features with repeatable options:

```shell
--delivery direct-to-trunk|pull-request|local-only
--enable worktrees|beads|agentic-systems|eval-case-reporting
--disable worktrees|beads|agentic-systems|eval-case-reporting
```

Enabled capabilities reconcile every external binary declared for them in
`bin/tool-releases.json`; Git and `tar` remain host prerequisites rather than
managed tools. For Beads, the current managed dependency is `bd >= 1.1.2`.
Beads uses its embedded Dolt engine by default, so do not require or install a
standalone `dolt` CLI unless the user explicitly chooses server mode or direct
database administration.

At Pi startup and in setup previews, show each missing, invalid, or outdated
managed tool with its current status, manifest target, `~/.local/bin`
destination, user-global scope, and no-sudo guarantee. A local-TUI user must
explicitly approve installation or update. Approval downloads only the pinned
HTTPS release, verifies SHA-256, installs atomically, verifies the resulting
version, and preserves a previous working executable on failure. Declining
keeps the capability unavailable; retry with
`/development-system-setup --enable beads`.

Always reconcile enabled dependencies even when the proposed TOML is bytewise
unchanged. For the `setup_no_changes` plus `beads: unavailable` regression,
explain the complete observable contract, not only the command:

- confirmation lists each tool's missing/outdated current state and manifest
  target version, `~/.local/bin`, user-global scope, and no-sudo requirement;
- approval installs the pinned, checksum-verified executable atomically;
- a tools-only result reports unchanged configuration separately and creates no
  repository commit;
- decline keeps Beads unavailable, and the same setup command reopens the offer;
- embedded mode needs no standalone Dolt CLI.

Make the user-global directory available immediately to extension child
processes. If it was absent from the inherited `PATH`, report the exact
`export PATH="$HOME/.local/bin:$PATH"` action and shell-restart requirement
rather than claiming complete shell integration. Setup then initializes
embedded Dolt storage without competing Git or harness hooks, sets Dolt
auto-commit, and installs formulas under `.beads/formulas/`.
The delivery mode selects `development-change-direct`,
`development-change-pr`, or `development-change-local` in project policy.

For a legacy Tiber project, preview
`development-system migrate-tiber-to-beads --dry-run` from the primary checkout.
After approval, apply with `--apply --yes`; add `--push` only when the Dolt remote
should be updated immediately. Then run setup to converge project policy and
formula files. Never retain or reinstall the Tiber runtime.
