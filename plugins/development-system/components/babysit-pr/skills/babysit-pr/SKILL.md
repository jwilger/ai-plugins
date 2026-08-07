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

For the survive-laptop-off case, this can be backed by a scheduled/cloud agent
(e.g. a Claude Cloud Routine) rather than a local loop.

## Notes

- Treat the project's default branch as protected: never push to it directly or
  merge manually when a required review/CI gate exists.
- One PR at a time: don't start new work while a PR is still mid-flight.
