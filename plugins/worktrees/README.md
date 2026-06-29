# worktrees

Make a repository worktree-ready, and protect the main checkout.

## Enforcement guard

`scripts/sidequest-guard.sh` blocks commits and pushes that originate from the
main checkout (rather than a linked worktree), so all changes flow through
worktrees (e.g. via `/side-quest`). It is deterministic and self-healing: an
attempt to commit in the main checkout fails with a message that points you to a
worktree. Install it as the repo's `pre-commit` and `pre-push` hooks:

```shell
cp scripts/sidequest-guard.sh .git/hooks/pre-commit
cp scripts/sidequest-guard.sh .git/hooks/pre-push
chmod +x .git/hooks/pre-commit .git/hooks/pre-push
```

The guard whitelists nothing it shouldn't: it only blocks commits/pushes, never
worktree creation or file edits, so the `/side-quest` machinery is never blocked.

A goal-driven `setup` skill (per-project isolation: ports, containers, caches,
secrets) is planned.

## Harnesses

Harness-agnostic — the guard and scripts are plain shell, used identically by
Claude Code and Codex.
