# Worktree-ready setup reference

Realize only the isolation goals that apply to the detected project stack:

1. Use an ignored repo-local `./.worktrees/` root unless the user selects
   another location. Before creating a worktree, verify the chosen root with
   `git check-ignore -q -- <root>`; stop and configure an ignore rule when that
   check fails.
2. Make untracked configuration available without copying secrets. Prefer an
   upward `.env` search plus a per-worktree override.
3. Copy or link warm build caches according to tool compatibility.
4. Namespace containers, databases, and volumes per worktree when services can
   overlap.
5. Allocate non-colliding port blocks with `scripts/worktree-ports.sh` and write
   the result into the worktree environment.
6. Tailor `templates/bootstrap-worktree.sh` and
   `templates/worktree-teardown.sh`; bootstrap after creation and tear services
   down before removal.

Detect the project's existing command surface before adding shortcuts:
`justfile`, `Makefile`, package scripts, another task runner, or direct shell
commands. Confirm the selected wrapper with the user before editing it. Generate
behavior tests for the tailored scripts and wrapper, document the optional
workflow in `AGENTS.md`, and run the repository's setup and baseline tests in a
new worktree before feature edits.

Cleanup is optional housekeeping. Never force-remove a dirty, detached,
identity-changed, or otherwise valuable worktree.
