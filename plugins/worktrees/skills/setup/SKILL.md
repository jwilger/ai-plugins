---
name: setup
description: Use when making a repository worktree-ready for parallel development — per-worktree isolation (ports, containers, caches, secrets), lifecycle hooks, and the main-checkout enforcement guard.
---

# Worktree-ready setup

Make a repository support isolated, parallel worktree development. This is
**goal-driven**: realize the isolation goals below for _this_ project's stack —
do not copy a fixed bootstrap. The reference scripts in this plugin are starting
points to tailor, not drop-in solutions.

## Isolation goals (realize the ones that apply; skip the rest)

1. **Filesystem** — each worktree is its own checkout (inherent to git
   worktrees). Use the ignored repo-local default checkout root
   `./.worktrees/` unless the user confirms another location.
2. **Untracked config / secrets** — the worktree needs working config without
   _copying secrets around_. Prefer an upward `.env` search (the app or `.envrc`
   walks up to the main checkout's `.env`) plus a per-worktree override file.
3. **Warm build caches** — copy or link build artifacts so a new worktree starts
   fast (`target/`, `_build/`, `deps/`, `node_modules/`, `.direnv/`, …). Copy
   (rsync) when tools dislike symlinked caches; link when they don't.
4. **Service / container isolation** — give each worktree its own containers, DB,
   and volumes by namespacing on a per-worktree project name (e.g. Docker Compose
   `COMPOSE_PROJECT_NAME`), so parallel worktrees never clobber each other.
5. **Port isolation** — assign each worktree a non-colliding block of host ports.
   Use `scripts/worktree-ports.sh <worktree>` (slot-based, idempotent,
   configurable bases) and write the result into the worktree's env.
6. **Lifecycle** — bootstrap on worktree creation, tear down before removal. Wire
   a `post-checkout` hook (lefthook/husky/plain git) to the bootstrap, and run
   teardown (`docker compose … down --volumes`, then `git worktree remove`)
   before deleting a worktree. Generate plain shell scripts first; convenience
   command wrappers are optional and must fit the project.
7. **Enforcement** — install the main-checkout guard so commits and pushes only
   originate from worktrees (this plugin's `scripts/worktree-guard.sh`).

## How to apply

1. Detect the stack: language, package manager, containers, dev server.
2. Decide which goals apply and how (which caches, which services, which ports).
3. Use `./.worktrees/<name>` as the default checkout path in generated examples
   and helper scripts. Ensure `.worktrees/` is ignored before relying on it.
4. Generate a project-specific bootstrap (adapt `templates/bootstrap-worktree.sh`)
   wired into the project's hook manager, and a teardown (adapt
   `templates/worktree-teardown.sh`).
5. Detect the project command surface before adding workflow shortcuts:
   `justfile`, `Makefile`, `package.json` scripts, project-specific task runners,
   or no wrapper at all. Do not assume `just`, npm, make, or any other runner is
   present across projects.
6. Present the detected command surface and confirm the selected wrapper with
   the user before editing it. If no wrapper is confirmed, document direct shell
   usage only.
7. Generate `bats` tests for the generated scripts and any confirmed wrapper.
8. Install the enforcement guard as the `pre-commit` and `pre-push` hooks.
9. Document the resulting workflow in the project's `AGENTS.md`.
