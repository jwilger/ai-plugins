# worktrees

Make a repository worktree-ready so separate concurrent mutable tasks can use
isolated checkouts without reserving the primary checkout.

By default, generated workflows place linked worktrees under the repo-local,
git-ignored `./.worktrees/` directory. The scripts do not require `just`, Make,
npm, or any other task runner: the setup skill detects what a project already
uses, confirms the selected wrapper with the user, and falls back to direct shell
usage when no wrapper should be added.

## Retired guard compatibility

`scripts/worktree-guard.sh` is a no-op compatibility shim for older managed
hooks. Do not install it. Primary-checkout commits and pushes are supported.

## Setup skill

The goal-driven `setup` skill (`skills/setup/`) makes a repository worktree-ready:
it defines the isolation goals (filesystem, secrets, warm caches, container/DB
isolation, ports, lifecycle hooks, enforcement) and guides tailoring a
project-specific bootstrap. Reusable building blocks: `scripts/worktree-ports.sh`
(slot-based, idempotent port allocator) and the `templates/` bootstrap/teardown
starting points. Generated convenience commands must adapt to the target repo's
existing command surface rather than assuming this marketplace repo's tooling.

The skill keeps one mutable task in the current checkout and creates a linked
worktree only for an explicit request or separate concurrent mutable work.

## Harnesses

Harness-agnostic — the guard and scripts are plain shell, used identically by
Claude Code and Codex.
