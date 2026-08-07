---
name: setup
description: Use when initializing or reconfiguring a repository with the development-system plugin, including delivery mode, Tiber, optional capabilities, and conflict checks.
---

# Development system setup

Setup is an explicit repository mutation and must run from the primary Git
checkout. Compare `git rev-parse --absolute-git-dir` with
`git rev-parse --path-format=absolute --git-common-dir`; if they differ, resolve
the primary checkout and stop rather than applying setup from the linked
worktree.

Use the plugin-owned command:

```shell
<plugin-root>/bin/development-system setup --project <repo> --preset personal-trunk --dry-run
```

Show the complete preview and ask for explicit approval before replacing
`--dry-run` with `--apply --yes`. Approval applies only to that preview; rerun
the preview and ask again after any option or detected-conflict change. Ask
customization questions one at a time.

The default is `--delivery direct-to-trunk` with `tiber` enabled. Linked
worktree support is always available and is not a configurable feature. Select
optional features with repeatable options:

```shell
--delivery direct-to-trunk|pull-request|local-only
--enable tiber|agentic-systems|eval-case-reporting
--disable tiber|agentic-systems|eval-case-reporting
```

There is no migration mode. Configure existing projects from scratch.
