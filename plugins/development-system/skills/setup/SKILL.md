---
name: setup
description: Use when initializing or reconfiguring a repository with the development-system plugin, including delivery mode, Tiber, optional capabilities, and conflict checks.
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

The default is `--delivery direct-to-trunk` with `tiber` enabled. Linked
worktree support is always available and is not a configurable feature. Select
optional features with repeatable options:

```shell
--delivery direct-to-trunk|pull-request|local-only
--enable tiber|agentic-systems|eval-case-reporting
--disable tiber|agentic-systems|eval-case-reporting
```

There is no migration mode. Configure existing projects from scratch.
