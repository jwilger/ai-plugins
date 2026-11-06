---
name: babysit-pr
description: Use when a pull request or merge request needs to be driven to merge — monitoring current-head checks, classifying failures, handling review threads, and merging only after repository-required gates, across GitHub, Forgejo, or GitLab.
---

# Babysit a PR/MR to merge

Watch a pull/merge request and drive it to merge. Forge-agnostic: detect the
forge with `scripts/detect-forge.sh` and use the matching tooling. None is
preferred over the others.

| Forge   | Tooling                                                  |
| ------- | -------------------------------------------------------- |
| GitHub  | `gh`                                                     |
| Forgejo | Forgejo MCP tools (`forgejo_*`); `tea` as a CLI fallback |
| GitLab  | `glab`                                                   |

## Local review and draft boundary

Commit small, coherent passing increments extremely frequently in every delivery mode; do not wait for a feature, vertical slice, or final MVP to be complete. Direct-to-trunk pushes go to origin’s configured trunk; PR/MR pushes go to the selected branch; local-only commits stay local. An explicit no-commit instruction takes precedence and uses the reviewed snapshot route. Each commit runs the configured pre-commit checks at minimum, preserves signing and exact source/message/signature verification, and receives proportional independent review before trunk publication. Review the actual increment and its affected dependencies; unfinished acceptance criteria for later increments are not publication blockers unless the current increment breaks an existing contract. Keep required CI gates and record the exact pushed SHA. Supporting commits are progress toward a slice, not delivered user value. Continue completing usable vertical slices regularly and perform scope-complete final review and acceptance before claiming feature or MVP completion.

In direct-to-trunk mode, use one checkout on the configured trunk branch and push directly to origin’s trunk. Do not create a branch, linked worktree, or parallel source checkout unless the user explicitly authorizes a different approach. Serialize mutable work in that checkout, including CI repair; read-only reviewers may inspect it. Use affected independent review for each increment rather than accumulating a completed local batch for one eventual publication review. Keep the immutable task-start baseline for final acceptance while recording each increment’s reviewed scope and exact identity separately.

In PR/MR mode, authorized branch pushes and draft creation may precede full local review. Before updating any ready PR/MR, first return it to draft and confirm success, then disable auto-merge and confirm it is off. Stop if either safeguard fails. Current local multi-lens review is required before restoring readiness, enabling auto-merge, or entering the merge queue. Local-only work remains local and uses the authorized commit or no-commit evidence route. In all modes, source, dependency, baseline, scope, or conflict-resolution changes invalidate affected evidence; metadata-only changes require exact identity verification. Required remote CI follows push, and a failure preempts unrelated work through CI recovery. Neither passing CI nor forge approval substitutes for required independent review.

Do not enable auto-merge or ready a PR without current local review evidence.
A draft may collect CI but cannot be treated as a ready delivery candidate.

## Loop (until merged, or blocked needing a human)

1. **Poll current-head status** — Track the PR/MR head revision, every required
   status check or pipeline job for that revision, the current review decision,
   unresolved change requests or review threads, mergeability, branch-update
   requirements, auto-merge, and merge-queue state. A new head revision
   invalidates prior check evidence and may invalidate reviews; re-evaluate the
   complete merge predicate after every push or rebase. Continue polling until
   the PR/MR is merged or an external-intervention blocker is proven. Use the
   harness's wait mechanism and honor forge rate-limit or retry guidance.
2. **On CI failure** — Fetch the failed job and step evidence and classify the
   failure as change-caused, transient/flaky, or infrastructure/external. Enter
   the repository's configured repair lifecycle and change code only for a
   causal failure. Use an authorized rerun for a transient failure; do not hide
   an infrastructure failure behind an unrelated code change.
3. **On review feedback** — apply the receiving-code-review discipline: verify
   each point against the code, push back with technical reasoning or implement
   it, and reply in the thread. For GitHub inline review comments, route through
   `github:gh-address-comments` so unresolved review threads, inline anchors,
   and resolution state are handled with thread-aware tooling. Do not post a top-level PR comment
   when the feedback came from an inline review thread; reply directly to that
   inline review thread, then resolve or re-request review once addressed.
4. **Merge** — Enable auto-merge or submit to the merge queue if the project
   allows it; otherwise merge only when all repository-required checks have
   succeeded for the current head revision, required reviews remain valid,
   required change requests and review threads are resolved, mergeability and
   branch-update rules are satisfied, and project policy permits the action.
   After enabling auto-merge or queueing, keep polling until the PR/MR state is
   merged.
5. **Waiting is not blocked** — Pending checks, bot reviews, auto-merge, and merge queue states are waiting states, not blockers.
   Do not stop merely because there is nothing to do yet.
6. **External-intervention blocker** — Stop and notify only when no authorized
   agent action or further polling can advance the state: for example, a
   required approval the agent cannot provide, a review only an authorized
   maintainer can dismiss, a missing signing key, an authentication prompt, or
   a permission boundary. State the exact gate and the action an authorized
   person must take.

For the survive-laptop-off case, this can be backed by a scheduled or cloud
agent rather than a local loop.

## Notes

- Treat the project's default branch as protected: never push to it directly or
  merge manually when a required review/CI gate exists.
- One PR at a time: don't start new work while a PR is still mid-flight.
