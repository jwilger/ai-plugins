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

Use the repository's declared trunk branch. Complete the required local checks
and final review, then make the normal authorized push without creating a
PR/MR. Preserve repository-required branch or worktree topology; direct-to-trunk
describes the delivery destination, not where development must occur. After pushing, bind the delivery evidence to the exact
pushed revision and verify its required CI run reaches the state the repository
requires. If a rejected push, rebase, merge, or conflict resolution changes the
candidate revision, the prior local evidence is stale: rerun the
repository-required checks and final review against the new revision before
retrying the push.

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
