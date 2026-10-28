---
name: delivery-workflow
description: Use when choosing whether and how verified work should be committed, pushed, reviewed, and delivered under a repository's local workflow policy; local-only mode does not authorize a commit unless the user or repository requires one.
---

# Delivery workflow

Apply `model-routing` whenever this workflow delegates delivery work. Keep
release, merge-readiness, and other completion decisions on the strong route
defined by that canonical matrix; model choice never supplies authorization.

Choose the delivery path from the repository's actual policy instead of assuming
that every change needs a pull request.

Passing a development or review gate records evidence; it does not authorize
a commit, push, PR/MR, merge, release, or destructive operation. Obtain action
authorization independently from current user direction or repository policy.

## Precedence

Current user direction comes first. Apply guidance in this order:

1. current user direction for this work;
2. repository-local instructions, such as `AGENTS.md`, repository rules, and
   checked-in workflow configuration;
3. this delivery-workflow router;
4. specialist skills for testing, review, CI recovery, and release details.

A current user restriction narrows standing repository authorization. A
local-only request therefore prevents a routine push even when the repository
normally authorizes direct-to-trunk delivery. A broader current request does not
override a repository safety restriction unless the user explicitly authorizes
that specific exception.

Specialist skills supplement the selected mode, commit cadence, and evidence
level. They must not replace them with a conflicting workflow or a gate that is
disproportionate to the repository policy and concrete risk. If the applicable
policy is genuinely ambiguous and the next externally visible action depends on
the answer, ask one concise question. When policy and authorization are clear,
do not ask again merely because an authorized action is first-time or
consequential. Do not invent a pull request.

## Select one mode

### Direct-to-trunk

Complete planned implementation and local verification before the full multi-lens review. Preserve signed, exact-verified local checkpoint commits as work progresses. In direct-to-trunk mode, review the completed local batch against the pinned baseline BEFORE pushing any of that batch to trunk; ordinary local increments do not each require full review. In PR/MR mode, authorized branch pushes and draft PR/MR creation may precede review, but creating a ready/non-draft PR/MR or marking a draft ready requires current local multi-lens review. Before pushing source changes to any existing ready PR/MR, FIRST return it to draft and confirm that transition succeeded, even when auto-merge is already off; only then disable auto-merge and confirm it is off. Do not push if either safeguard fails. After pushing, renew affected review before marking it ready, enabling auto-merge, or entering a merge queue. Source changes, baseline changes, scope changes, and conflict resolution invalidate affected review evidence; metadata-only identity changes require fresh exact verification. Local-only review uses the verified local identity and requires no remote CI. Remote CI follows push and remains a separate exact-SHA readiness gate; a failed build preempts work through CI recovery. No passing build, forge approval, or auto-merge substitutes for local multi-lens review.

Signed local checkpoint commits run the repository pre-commit hook once; pushes
run its pre-push hook once. Exact verification checks source, message and
signature without duplicating hook-owned checks. Repairs use additive local
commits, affected review, then the selected publication boundary.

If a pre-commit hook fails without creating a commit, first record its actual
failed command and evidence as canonical `failing`, with unchanged HEAD,
`failure_kind: pre-commit-hook`, cleared gate receipts, null delivery, and one
causal repair. A lightweight review requiring remediation uses the same
failure transition with `failure_kind: lightweight-review`. Follow the public
`development-workflow` schema and stranded-checkpoint helper; never fabricate
a commit, reuse passing gate receipts, discard staged work, or replace the
checkpoint manually. Only the recorded causal repair is permitted while
failing; fresh focused testing and fresh lightweight review must precede
another normal hook-backed commit attempt. Exact verification and the
configured delivery/CI/final-review gates still apply.

Keep checkpoint CI bounded per pushed ref. Prefer the forge or repository's
existing coalescing of superseded non-terminal runs; cancel an obsolete run
only when cancellation is authorized. If neither is available, wait for
capacity before another push would create obsolete queued work. Never cancel
the current terminal candidate's exact-SHA run, and never use a successful run
for an older SHA as readiness evidence for a newer checkpoint.

### PR/MR

Complete planned implementation and local verification before the full multi-lens review. Preserve signed, exact-verified local checkpoint commits as work progresses. In direct-to-trunk mode, review the completed local batch against the pinned baseline BEFORE pushing any of that batch to trunk; ordinary local increments do not each require full review. In PR/MR mode, authorized branch pushes and draft PR/MR creation may precede review, but creating a ready/non-draft PR/MR or marking a draft ready requires current local multi-lens review. Before pushing source changes to any existing ready PR/MR, FIRST return it to draft and confirm that transition succeeded, even when auto-merge is already off; only then disable auto-merge and confirm it is off. Do not push if either safeguard fails. After pushing, renew affected review before marking it ready, enabling auto-merge, or entering a merge queue. Source changes, baseline changes, scope changes, and conflict resolution invalidate affected review evidence; metadata-only identity changes require fresh exact verification. Local-only review uses the verified local identity and requires no remote CI. Remote CI follows push and remains a separate exact-SHA readiness gate; a failed build preempts work through CI recovery. No passing build, forge approval, or auto-merge substitutes for local multi-lens review.

Opening, updating and merging PR/MRs remain separately authorized. Bind checks,
review, approval and merge state to the exact current head revision.

### Local-only

Keep all work local and do not push, open a PR/MR, or merge. Commit only when
requested or required by the applicable repository policy, and only with commit
authority. That route uses the normal commit hook and exact signed commit.
Otherwise use the authorized exact no-commit snapshot route with fresh,
proportionate local checks and review; never fabricate a commit-hook receipt or
manually duplicate hook-owned checks around an authorized commit. An explicit
committed-evidence policy, including Tiber's opt-in minimum-clean-reviews policy,
still requires evidence in a commit tree: without commit authority, completion
blocks rather than substituting a snapshot or performing remote work.

Record the exact verified local identity, completed gates and next permitted
action as the delivery-mode equivalent checkpoint. Report local checks and
working-tree state without remote CI claims. Final review still applies at
ticket completion with fresh local evidence. A clean unchanged review creates
no commit. Source-changing remediation renews local verification and affected
review through the same authorized commit or no-commit route.

## Authorization and evidence

- Before creating any authorized commit, use `rationale-commit-messages`. Every
  authored commit requires a concise Conventional Commit subject and a
  non-empty body explaining why the change exists. Capture the motivation,
  decision context, tradeoff, or failure being prevented; reject a subject-only
  message and a body that merely restates the subject or diff.
- After creating the commit, prove that its paths, contents, and modes are
  identical to the lightweight-reviewed checkpoint snapshot. Verify its message
  and signature against the exact commit. Do not rerun pre-commit or pre-push
  verification manually; the Lefthook receipts from the Git operations are the
  evidence. Comprehensive, slow, integration, mutation, and pipeline-scale
  checks belong in CI unless failure diagnosis requires them locally.
  The stage-aware review hash may change at this boundary without reopening
  review; source identity, not Git partition or metadata identity, decides that
  question. Before terminal completion, preserve the stage-aware contract:
  recompute and submit the fresh `current_diff_hash` on every review advance and
  handle every mismatch through final-review rather than this delivery exception.
- Treat a user request or standing repository authorization as permission only
  for the externally visible operations it actually covers.
- Use additive commits for repairs, review follow-ups, and later corrections by
  default, then use the mode's normal push. A request to fix feedback, keep a
  PR tidy, or perform routine delivery does not authorize rewriting history.
- Require explicit case-by-case user authorization before amending any existing
  commit, whether or not it has been pushed. If the amended commit would also
  require a forced push, authorize that forced push separately; approval for
  one operation does not imply approval for the other.
- Never amend shared or default-branch history as a routine repair. Preserve it
  with a new corrective commit and a normal push. On any other branch, amend
  only when the user explicitly authorizes that specific amend.
- Require explicit case-by-case approval for destructive or irreversible
  operations, including forced pushes, even when routine delivery is authorized.
- Keep testing, review, and CI evidence proportional to the change's risk and to
  the completion claim. A pushed CI failure invokes `ci-failure-follow-up` and
  blocks final review, readiness claims, and unrelated work until replacement
  evidence reaches terminal success; no exception can weaken that hold, and
  switching delivery modes cannot hide or bypass it.
- For direct-to-trunk delivery, report the exact pushed revision and, when
  repository policy requires CI, its terminal result. When it does not, state
  plainly that no remote CI evidence is required. For PR/MR delivery, report the
  exact head revision plus required checks, review, approval, and merge state.
  For local-only work, report the local checks, review result, and working-tree
  state without implying delivery.

Missing evidence remains missing; never manufacture a PR, CI requirement, or
remote action merely to make the modes look alike.
