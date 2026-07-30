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

When Beads is enabled, require `bd >= 1.0.0`, initialize its embedded Dolt
backend without installing competing Git or harness hooks, set Dolt auto-commit,
and install the development-system workflow formulas under `.beads/formulas/`.
The delivery mode selects `development-change-direct`,
`development-change-pr`, or `development-change-local` in project policy.

For a legacy Tiber project, preview
`development-system migrate-tiber-to-beads --dry-run` from the primary checkout.
After approval, apply with `--apply --yes`; add `--push` only when the Dolt remote
should be updated immediately. Then run setup to converge project policy and
formula files. Never retain or reinstall the Tiber runtime.
