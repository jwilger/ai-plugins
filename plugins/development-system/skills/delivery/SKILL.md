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

Before creating any authorized commit, write a concise Conventional Commit
subject and a non-empty body that explains why the change exists. The body must
capture the motivation, decision context, tradeoff, or failure being prevented;
it must not merely restate the subject or summarize the diff. Reject a
subject-only message. Never add `Co-Authored-By` or another AI-attribution
trailer.

Always state each protected action explicitly: never infer permission to
force-push, merge, or delete remote state.
