#!/usr/bin/env bash
# Retired compatibility shim for previously installed managed hooks.
set -euo pipefail

# Worktrees are optional concurrent-work isolation. Deliberately do nothing so
# old launchers stop blocking before the shared hooks are refreshed.
