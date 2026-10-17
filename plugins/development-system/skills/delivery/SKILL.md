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

- A semantic commit operation must consume a
  `passing-awaiting-gates-or-review` per-edit checkpoint whose bounded
  lightweight review has completed. The commit operation itself must trigger
  the Lefthook pre-commit gate and return both its gate receipt and a signed
  commit receipt when signing is required. Never run the same gate separately
  before committing.
- A failed pre-commit attempt that created no commit, or a lightweight review
  requiring remediation, must first record canonical `failing` with actual
  failure evidence through the public `development-workflow` state machine.
  Commit and push are prohibited while failing. Preserve staged work; permit
  only the causal repair followed by fresh tests, review, and a new normal
  hook-backed commit attempt. Use its recovery helper for a stranded record,
  never manual checkpoint replacement or fabricated passing receipts.
- A semantic tag operation is separate from an increment commit: it consumes
  the repository-authorized delivered revision and release evidence required
  by policy, and returns a tag receipt when signing is required.
- A semantic push operation must trigger the Lefthook pre-push gate and return
  its verification plus an idempotent remote receipt. Never run that gate
  separately before pushing. PR/merge operations return their idempotent remote
  receipts.
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

Load the retained `delivery-workflow` contract before the first increment.

Complete planned implementation and local verification before the full multi-lens review. Preserve signed, exact-verified local checkpoint commits as work progresses. In direct-to-trunk mode, review the completed local batch against the pinned baseline BEFORE pushing any of that batch to trunk; ordinary local increments do not each require full review. In PR/MR mode, authorized branch pushes and draft PR/MR creation may precede review, but creating a ready/non-draft PR/MR or marking a draft ready requires current local multi-lens review. Before pushing source changes to an existing ready PR/MR, disable auto-merge and return it to draft, then renew affected review before making it ready again. Keep the PR draft if either operation fails. Source changes, baseline changes, scope changes, and conflict resolution invalidate affected review evidence; metadata-only identity changes require fresh exact verification. Local-only review uses the verified local identity and requires no remote CI. Remote CI follows push and remains a separate exact-SHA readiness gate; a failed build preempts work through CI recovery. No passing build, forge approval, or auto-merge substitutes for local multi-lens review.

Keep checkpoint CI bounded per pushed ref. Use forge/repository coalescing for
superseded non-terminal runs, or cancel an obsolete run only when cancellation
is authorized; otherwise wait for capacity before another push would create
obsolete queued work. Protect the current terminal candidate's exact-SHA run
from cancellation, and reject a successful older-SHA run as readiness evidence
for the newer checkpoint.

If the same-MCP final-review protocol preflight reports a stale skill/runtime
or coordinator mismatch, reject terminal delivery, PR/MR creation or update,
merge, and readiness claims until the required recovery succeeds. That
mismatch does not retroactively forbid a separately authorized intermediate or
terminal-finding-remediation checkpoint: it may still preserve a local commit or push a draft branch; trunk push and
PR readiness remain blocked, but it cannot be called terminal delivery or readiness evidence.

Always state each protected action explicitly: never infer permission to
force-push, merge, or delete remote state.
