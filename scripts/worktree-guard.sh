#!/usr/bin/env bash
# Retired compatibility shim for previously installed managed Git hooks.
#
# Linked worktrees are now used only to isolate concurrent mutable tickets, so
# the primary checkout is no longer universally guarded. The hook installer
# removes the old managed pre-commit and pre-push launchers on its next run.
set -euo pipefail

# Deliberately do nothing: this file keeps a checkout usable between upgrading
# tracked files and rerunning `just worktree-hooks`.
