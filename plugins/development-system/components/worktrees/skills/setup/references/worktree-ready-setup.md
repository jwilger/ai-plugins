# Worktree-ready setup reference

Realize only the isolation goals that apply to the detected project stack:

1. Use an ignored repo-local `./.worktrees/` root unless the user selects
   another location. Before creating a worktree, verify the chosen root with
   `git check-ignore -q -- <root>`; stop and configure an ignore rule when that
   check fails.
2. Load secrets from one untracked source and layer a per-worktree non-secret
   override with documented precedence. Do not copy secrets into generated or
   tracked files.
3. Share only immutable or content-addressed download caches. Copy or namespace
   writable build caches and state so concurrent worktrees cannot corrupt one
   another.
4. Give every worktree a recorded service namespace: include the Compose
   project, database or schema, writable volumes, Unix sockets, and other
   process-global names that can collide.
5. Allocate non-colliding port blocks with `scripts/worktree-ports.sh` and write
   the result into the worktree environment.
6. Tailor `templates/bootstrap-worktree.sh` and
   `templates/worktree-teardown.sh`; bootstrap after creation and tear down only
   the services and resources named by that worktree's recorded namespace before
   removal.

Detect the project's existing command surface before adding shortcuts:
`justfile`, `Makefile`, package scripts, another task runner, or direct shell
commands. Confirm the selected wrapper with the user before editing it. Generate
behavior tests for the tailored scripts and wrapper, document the optional
workflow in `AGENTS.md`, and run the repository's setup and baseline tests in a
new worktree before feature edits.

Cleanup is optional housekeeping. Refuse removal when the worktree is dirty,
contains a detached or unpublished commit, no longer matches its recorded
path/branch/HEAD identity, still owns running namespaced services, or teardown
failed. Never force-remove valuable state.
