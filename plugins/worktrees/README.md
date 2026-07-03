# worktrees

Make a repository worktree-ready, and protect the main checkout.

## Enforcement guard

`scripts/worktree-guard.sh` blocks commits and pushes that originate from the
main checkout (rather than a linked worktree), so all changes flow through
linked worktrees. It is deterministic and self-healing: an attempt to commit in
the main checkout fails with a message that points you to a worktree. Install it
as the repo's `pre-commit` and `pre-push` hooks:

```shell
cp scripts/worktree-guard.sh .git/hooks/pre-commit
cp scripts/worktree-guard.sh .git/hooks/pre-push
chmod +x .git/hooks/pre-commit .git/hooks/pre-push
```

The guard whitelists nothing it shouldn't: it only blocks commits/pushes, never
worktree creation or file edits.

## Setup skill

The goal-driven `setup` skill (`skills/setup/`) makes a repository worktree-ready:
it defines the isolation goals (filesystem, secrets, warm caches, container/DB
isolation, ports, lifecycle hooks, enforcement) and guides tailoring a
project-specific bootstrap. Reusable building blocks: `scripts/worktree-ports.sh`
(slot-based, idempotent port allocator) and the `templates/` bootstrap/teardown
starting points.

## Harnesses

Harness-agnostic — the guard and scripts are plain shell, used identically by
Claude Code and Codex.
