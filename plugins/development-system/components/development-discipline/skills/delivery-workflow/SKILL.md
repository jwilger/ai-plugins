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

Development Discipline `workflow.authorize_delivery` is a mechanical phase
transition proving that its predecessor gates completed. It does not authorize
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

Use the repository's declared trunk branch. For every passing per-edit
checkpoint, including a standalone test-file edit, complete its immediate
focused test, bounded lightweight review, fast pre-commit gate, and signed
commit, then make the authorized
non-history-rewriting push immediately to the policy-selected trunk ref without creating a
PR/MR. Preserve repository-required branch or worktree topology; direct-to-trunk
describes the delivery destination, not where development must occur. After pushing, bind the delivery evidence to the exact
pushed revision and monitor its required CI concurrently with the next
increment. A completed failure preempts that work through CI recovery. Reserve
the full final review for ticket completion, when it consumes the last delivered
pushed identity. A clean unchanged terminal review creates no additional commit
or push. Each checkpoint commit requires fresh exact-commit verification of the
repository-required fast non-duplicated checks plus commit-message and signature
validation before push; comprehensive suites remain in CI. If a rejected push,
rebase, merge, conflict resolution, hook, formatter,
or other delivery step changes any reviewed path, content, mode, formerly
untracked content, adds a newly in-scope untracked path, changes the pinned
baseline, or changes the requested scope, the prior review is
stale: treat the mutation as a new causal checkpoint, rerun its immediate test,
lightweight review, fast gate, signed commit, exact verification, and normal
push. Run terminal delta/reset review against that new delivered identity only
when terminal review was already active or the mutation remediates a
terminal-review finding. During an ordinary checkpoint before ticket
completion, deliver the new causal checkpoint and continue with the next
planned increment; do not start terminal review early.
A metadata-only revision change stays in delivery verification and does not
reopen source review.

Keep checkpoint CI bounded per pushed ref. Prefer the forge or repository's
existing coalescing of superseded non-terminal runs; cancel an obsolete run
only when cancellation is authorized. If neither is available, wait for
capacity before another push would create obsolete queued work. Never cancel
the current terminal candidate's exact-SHA run, and never use a successful run
for an older SHA as readiness evidence for a newer checkpoint.

### PR/MR

Use a branch and the repository's pull-request or merge-request process. Push
each passing per-edit checkpoint only after its immediate focused test,
lightweight review, repository fast gate, and signed branch commit, and only
when that branch mutation is already authorized;
do not infer permission to open, update, or merge a PR/MR. Honor
its required checks, review, approval, merge queue, and cleanup rules. Opening,
updating, or merging the PR/MR must be authorized by the user or repository
policy. Start terminal review only after every planned increment and acceptance
criterion is checkpoint-delivered. It consumes the exact already-pushed branch
identity; a clean
unchanged review creates no push. Source-changing remediation uses a signed
additive commit and authorized branch push without inferring PR operations.
Bind every check, approval, review, and readiness claim to the PR/MR's exact
current head revision.

### Local-only

Keep all work local. Run checks and review in proportion to the claim, but do not
push, open a PR/MR, or merge. Do not commit by default: commit only when the
user authorizes it or repository-local instructions require it. If the
repository requires a commit but the user explicitly withholds commit
authority, block without committing or pushing. Record the exact locally authorized terminal
snapshot, completed gates, and next permitted action as the delivery-mode
equivalent checkpoint, and report remaining remote work plainly. Final review
still applies at ticket completion in local-only mode: run it with
fresh local evidence, and do not dismiss it as a publication-only or PR-only
gate. A clean unchanged terminal review creates no commit. Source-changing
remediation creates a signed local commit only when authorized. When committed
evidence is repository-required but commit authority is explicitly withheld,
block without committing or pushing; otherwise record a new exact reviewed,
fast-gate-passing no-commit snapshot.

## Authorization and evidence

- Before creating any authorized commit, use `rationale-commit-messages`. Every
  authored commit requires a concise Conventional Commit subject and a
  non-empty body explaining why the change exists. Capture the motivation,
  decision context, tradeoff, or failure being prevented; reject a subject-only
  message and a body that merely restates the subject or diff.
- After creating the commit, prove that its paths, contents, and modes are
  identical to the lightweight-reviewed checkpoint snapshot, or to the
  terminally reviewed source snapshot at ticket completion. Verify its message
  and signature against the exact commit. Do not insert a duplicate
  comprehensive local exact-commit suite after the repository fast pre-commit
  gate; comprehensive, slow, integration, mutation, and pipeline-scale checks
  belong in CI unless failure diagnosis requires them locally.
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
