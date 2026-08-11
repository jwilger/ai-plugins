---
name: delivery
description: Use when committing, pushing, opening or babysitting a PR/MR, merging, or deciding whether verified work is complete under the configured delivery mode.
---

# Delivery

Use the configured semantic repository-local and repository-remote services
when running inside standalone Tiber. In an ordinary host harness those
services are advisory components, so use the host's normal Git or forge tools
when the user has authorized delivery. Do not claim that the plugin denies
those tools or supplies authoritative delivery receipts.

For ongoing PR/MR monitoring through merge or an external-intervention blocker,
load the retained [babysit PR
contract](../../components/babysit-pr/skills/babysit-pr/SKILL.md).

When the configured value is unavailable, do not choose a mode. State that
`.development-system.toml` is authoritative and summarize all three modes so
the user knows what evidence is missing.

- A semantic commit/tag operation must consume a clean workflow checkpoint and
  return a signed receipt when signing is required.
- A semantic push/PR/merge operation must return an idempotent remote receipt.
- Failed pushed CI is blocking recovery work; the CI-recovery service performs
  only typed recovery actions.

For a PR/MR check failure, inspect the failed job and step before acting.
Classify it as change-caused, transient/flaky, or infrastructure/external; route
a causal failure through the configured repair lifecycle, use an authorized
rerun for a transient failure, and do not disguise infrastructure failure with
an unrelated code edit.

Before creating any authorized commit, write a concise Conventional Commit
subject and a non-empty body that explains why the change exists. The body must
capture the motivation, decision context, tradeoff, or failure being prevented;
it must not merely restate the subject or summarize the diff. Reject a
subject-only message. Never add `Co-Authored-By` or another AI-attribution
trailer.

Treat final source review, commit, and push as a one-way delivery boundary. A
commit that contains exactly the terminally reviewed paths, contents, and modes
does not trigger another full source review solely because staging state,
`HEAD`, signature, identities, timestamps, or other commit metadata changed.
Coordinator event, snapshot, and state bookkeeping likewise never enters the
reviewed source inventory or hash. Still run the repository-required fresh
post-commit full gate against the exact commit, verify its message and signature,
then push and bind required CI to that revision.
If delivery changes a reviewed path, content, mode, formerly untracked content,
adds a newly in-scope untracked path, changes the pinned baseline, or changes
the requested scope, return to final review for the changed
source snapshot. This boundary applies only after terminal review; while review
is active, keep the stage-aware helper unchanged and submit its fresh
`current_diff_hash` on every advance.

Always state each protected action explicitly: never infer permission to
force-push, merge, or delete remote state.
