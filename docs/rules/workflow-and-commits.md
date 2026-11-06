# Workflow, commits, and delivery

- **Repository-local delivery policy wins.** Route delivery through
  `development-discipline:delivery-workflow`: current user direction comes
  first and may narrow standing authorization, then repository-local
  instructions select direct-to-trunk, PR/MR, or local-only mode before
  specialist testing, review, CI, or release guidance is applied. A specialist
  may add proportionate gates within that mode but may not replace its workflow,
  commit cadence, or evidence level, or invent a pull request.
- **One major change per worktree at a time.** In PR mode, do not start another
  major task in that worktree while its PR is still waiting on CI, review,
  approval, merge, or local cleanup.
- **Trunk push CI by default; PR CI when explicitly configured.** Direct pushes
  to the default branch run the full gate. PR mode also uses merge-queue CI and
  requires **≥1 approval** plus **automated code review/approval**. CI gates:
  formatting, tests, and Codex marketplace and manifest verification. A newer
  revision cancels obsolete CI only within the same
  workflow, event type, and Git ref, keeping direct pushes, pull requests, and
  merge-queue validation isolated. Delivery evidence still binds to the exact
  latest pushed revision and waits for its terminal result.
- **Failed pushed CI holds unrelated work.** Use
  `development-discipline:ci-failure-follow-up`: inspect the exact failed
  job, step, and logs; record the causal diagnosis; then either push only the
  diagnosed repair with its rationale or, for an evidence-backed unrelated or
  transient classification, rerun the unchanged revision with no intervening
  push. Wait for that replacement run's terminal success before resuming
  unrelated implementation.
- **Rationale-bearing Conventional Commits.** Commit between BDD steps once
  `just pre-commit` is green. CI runs the full, expensive gate after the push.
  Every authored commit needs a concise Conventional Commit
  subject and a non-empty body explaining why the change is necessary, such as
  the motivation, tradeoff, or failure it prevents. A subject-only message, or
  a body that merely repeats what changed, is not complete.
- **One-hour scope check.** As a rough heuristic, if no commit has been pushed
  in the past hour, pause and ask whether the current increment is being
  over-engineered. Prefer a smaller semantic increment when possible; the
  heuristic never permits skipping tests, review, or another required gate.
- **No `Co-Authored-By` trailers** (and no other AI-attribution trailers).
- **Forge-agnostic, no preference:** GitHub, Forgejo, and GitLab are first-class
  peers. Use the forge tooling that matches the repository remote.
- **Document every architectural decision** as an ADR in `docs/adr/` (why,
  alternatives considered, and the conditions under which to revisit).

## Review before publication

Publish small, coherent passing increments extremely frequently; do not wait for a feature, vertical slice, or final MVP to be complete. Each commit runs the configured pre-commit checks at minimum, preserves signing and exact source/message/signature verification, and receives proportional independent review before trunk publication. Review the actual increment and its affected dependencies; unfinished acceptance criteria for later increments are not publication blockers unless the current increment breaks an existing contract. Keep required CI gates and record the exact pushed SHA. Supporting commits are progress toward a slice, not delivered user value. Continue completing usable vertical slices regularly and perform scope-complete final review and acceptance before claiming feature or MVP completion.

In direct-to-trunk mode, use one checkout on the configured trunk branch and push directly to origin’s trunk. Do not create a branch, linked worktree, or parallel source checkout unless the user explicitly authorizes a different approach. Serialize mutable work in that checkout, including CI repair; read-only reviewers may inspect it. Use affected independent review for each increment rather than accumulating a completed local batch for one eventual publication review. Keep the immutable task-start baseline for final acceptance while recording each increment’s reviewed scope and exact identity separately.

In PR/MR mode, authorized branch pushes and draft creation may precede full local review. Before updating any ready PR/MR, first return it to draft and confirm success, then disable auto-merge and confirm it is off. Stop if either safeguard fails. Current local multi-lens review is required before restoring readiness, enabling auto-merge, or entering the merge queue. Local-only work remains local and uses the authorized commit or no-commit evidence route. In all modes, source, dependency, baseline, scope, or conflict-resolution changes invalidate affected evidence; metadata-only changes require exact identity verification. Required remote CI follows push, and a failure preempts unrelated work through CI recovery. Neither passing CI nor forge approval substitutes for required independent review.

The one-hour push heuristic never bypasses this review boundary.
