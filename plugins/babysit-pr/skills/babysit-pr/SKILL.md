---
name: babysit-pr
description: Use when a pull request or merge request needs to be driven to merge — watching CI, responding to review feedback, and merging once green and approved, across GitHub, Forgejo, or GitLab.
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

1. **Poll status** — CI checks, review threads, mergeability, auto-merge, and
   merge queue state. Continue polling until the PR/MR is merged or a concrete
   unfixable human gate is proven.
2. **On CI failure** — fetch the logs, diagnose, and push a fix to the branch.
3. **On review feedback** — apply the receiving-code-review discipline: verify
   each point against the code, push back with technical reasoning or implement
   it, and reply in the thread. For GitHub inline review comments, route through
   `github:gh-address-comments` so unresolved review threads, inline anchors,
   and resolution state are handled with thread-aware tooling. Do not post a top-level PR comment
   when the feedback came from an inline review thread; reply directly to that
   inline review thread, then resolve or re-request review once addressed.
4. **Merge** — enable auto-merge or submit to the merge queue if the project
   allows it; otherwise merge once the PR is green and approved. After enabling
   auto-merge or queueing, keep polling until the PR/MR state is merged.
5. **Waiting is not blocked** — Pending checks, bot reviews, auto-merge, and merge queue states are waiting states, not blockers.
   Do not stop merely because there is nothing to do yet.
6. **Blocked needing a human** — stop and notify only for a concrete unfixable human gate:
   a required approval you cannot provide, a stale review you cannot dismiss, a
   missing signing key, an auth prompt, a permission boundary, or another gate
   that repeated polling cannot change. State the exact gate and the action a
   human must take.

For the survive-laptop-off case, this can be backed by a scheduled/cloud agent
(e.g. a Claude Cloud Routine) rather than a local loop.

## Notes

- Treat the project's default branch as protected: never push to it directly or
  merge manually when a required review/CI gate exists.
- One PR at a time: don't start new work while a PR is still mid-flight.
