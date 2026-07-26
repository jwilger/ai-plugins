---
name: delivery
description: Use when committing, pushing, opening or babysitting a PR/MR, merging, or deciding whether verified work is complete under the configured delivery mode.
---

# Delivery

Read `[delivery]` in `.development-system.toml`.

- `direct-to-trunk`: commit verified semantic increments and push the configured
  trunk branch. Treat failed pushed CI as blocking recovery work.
- `pull-request`: publish a branch, create or update one PR/MR, monitor required
  checks and review feedback, and finish only at the repository's configured
  terminal state.
- `local-only`: do not commit or publish unless explicitly authorized.

Never infer permission to force-push, merge, or delete remote state.
