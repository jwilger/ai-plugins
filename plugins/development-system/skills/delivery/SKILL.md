---
name: delivery
description: Use when committing, pushing, opening or babysitting a PR/MR, merging, or deciding whether verified work is complete under the configured delivery mode.
---

# Delivery

Read `[delivery]` in `.development-system.toml`.

When the configured value is unavailable, do not choose a mode. State that
`.development-system.toml` is authoritative and summarize all three modes so
the user knows what evidence is missing.

- `direct-to-trunk`: commit verified semantic increments and push the configured
  trunk branch. Treat failed pushed CI as blocking recovery work.
- `pull-request`: publish a branch, create or update one PR/MR, monitor required
  checks and review feedback, and finish only at the repository's configured
  terminal state.
- `local-only`: do not commit or publish unless explicitly authorized.

Always state each protected action explicitly: never infer permission to
force-push, merge, or delete remote state.

When worktrees are enabled, delivery is not complete while an agent-created
linked worktree is needlessly retained. After terminal delivery evidence and a
clean status, call `development_system_worktree_finish` to return the Pi session
to the primary checkout, run repository teardown, and remove the worktree while
preserving its branch. A dirty worktree or cleanup failure remains unresolved;
never force removal.
