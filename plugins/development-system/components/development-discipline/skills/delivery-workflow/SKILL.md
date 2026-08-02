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

Use the repository's declared trunk branch. For each intermediate verified
semantic checkpoint, run the fast local checks and lightweight review, then make
the normal authorized push without creating a PR/MR. Long-running integration,
mutation, exhaustive, full-suite, and similarly expensive checks belong in CI
unless a local run is directly required to diagnose a failure. Reserve full
final review for a completed-ticket candidate after its acceptance criteria are
implemented.
Preserve repository-required branch or worktree topology; direct-to-trunk
describes the delivery destination, not where development must occur. Another
verified semantic checkpoint may be pushed when the most recent completed run
for the configured trunk branch that reached a pass/fail outcome passed and no
unresolved failure hold exists. Newer queued, pending, or running runs do not
replace that watermark. A canceled run is non-evidence: it neither passes nor
fails, does not replace the watermark, and does not create or release a hold.
This checkpoint rule does not complete the ticket: bind final delivery evidence
to the exact final pushed SHA and require its CI run to reach terminal success.
If a rejected push, rebase, merge, or conflict resolution changes the candidate
revision, the prior local evidence is stale. Before retrying, rerun the gate
appropriate to that candidate: fast checks and lightweight review for an
intermediate checkpoint, or the repository-required local checks and full final
review for a completed-ticket candidate.

### PR/MR

Use a branch and the repository's pull-request or merge-request process. Honor
its required checks, review, approval, merge queue, and cleanup rules. Opening,
updating, or merging the PR/MR must be authorized by the user or repository
policy. Bind every check, approval, review, and readiness claim to the PR/MR's
exact current head revision.

### Local-only

Keep all work local. Run checks and review in proportion to the claim, but do not
push, open a PR/MR, or merge. Do not commit by default: commit only when requested or required by the
repository-local instructions, and report the local evidence and remaining
remote work plainly. Final review still applies in local-only mode: run it with
fresh local evidence, and do not dismiss it as a publication-only or PR-only
gate.

## Authorization and evidence

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
  the completion claim. An unexpected pushed-CI failure invokes
  `ci-failure-follow-up`; in direct-to-trunk mode, the hold starts the instant a
  completed run for the configured trunk branch fails unexpectedly. It blocks
  final review, readiness claims, unrelated work, and unrelated pushes until
  terminal success of either the exact tested causal-repair revision or the
  authorized rerun of the exact unchanged failed SHA. No exception can weaken
  that hold, and switching delivery modes cannot hide or bypass it.
- For direct-to-trunk delivery, report the exact pushed revision and, when
  repository policy requires CI, require terminal success for that exact final
  SHA before reporting ticket completion. When it does not, state plainly that
  no remote CI evidence is required. For PR/MR delivery, report the exact head
  revision plus required checks, review, approval, and merge state. For
  local-only work, report the local checks, review result, and working-tree state
  without implying delivery.

Missing evidence remains missing; never manufacture a PR, CI requirement, or
remote action merely to make the modes look alike.
