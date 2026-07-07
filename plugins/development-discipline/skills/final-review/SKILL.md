---
name: final-review
description: Use when preparing local changes, a branch, pull request, merge request, or merge-to-main for final review before publishing, opening a PR, merging, or claiming readiness.
---

# Final Review

Run a local, fresh-context review cycle before creating a pull request, merging,
or claiming a change is ready.

## Scope

Resolve the reviewed diff from the user's requested scope. Always check current
branch and worktree status first.

| User asks for                  | Review scope                                                                 |
| ------------------------------ | ---------------------------------------------------------------------------- |
| No explicit base               | `origin/main...HEAD`, plus index/worktree changes unless explicitly excluded |
| Uncommitted changes            | working tree plus index against `HEAD`                                       |
| Since a branch, tag, or commit | that ref against `HEAD` unless the user says otherwise                       |
| Existing PR/MR                 | PR/MR base against its head                                                  |

If the base is ambiguous, infer the safest local scope and state it. Do not
review unrelated local dirt outside the requested scope. If dirty local changes
exist and are excluded, say so explicitly before any readiness claim.

## Default Lenses

Use repository-agnostic lenses by default:

- `correctness-behavior`: requirements, edge cases, regressions, and observable behavior.
- `tests-verification`: test quality, missing coverage, stale evidence, and whether verification proves the claim.
- `security-safety`: secrets, injection, permissions, unsafe subprocess/file/network behavior, and trust boundaries.
- `architecture-maintainability`: fit with local patterns, coupling, complexity, naming, and future change cost.
- `operability-user-impact`: failure messages, ergonomics, configuration, migration, observability, and recovery.
- `release-integration`: versioning, compatibility, packaging, docs, CI, rollout, and downstream integration.
- `production-risk-footguns`: latent traps, fragile defaults, data-access or resource-use patterns that pass lower environments but fail at production scale, and burst/DOS-like load behavior.

Add conditional lenses only when the diff calls for them, such as accessibility
for UI work, data correctness for analytics, or agent-instruction quality for
skill/prompt/plugin changes.

## Loop

1. Derive the baseline and capture the diff for the reviewed scope.
2. For each lens, launch a fresh-context review subagent for the current
   iteration. Give it only the baseline, diff, relevant files, and any prior
   defenses for that lens.
3. Classify findings as valid, invalid, or intentionally not addressed.
4. If the user asked for remediation, fix valid findings. If the user asked for
   review only, do not edit files; report valid findings as required follow-up.
   For findings not addressed, write a concise technical defense tied to the
   relevant lens.
5. Repeat with fresh subagents. Pass prior defenses back to the relevant lens so
   the next reviewer can challenge them.
6. Stop only after three consecutive full iterations where no lens raises any
   finding. A defended finding counts as clean only after the next relevant
   lens accepts the defense without raising it again.

This skill requires a harness that can launch fresh-context subagents. If that
capability is unavailable, stop and report that final-review cannot be completed
to its standard instead of silently replacing it with an in-context review.

## Output

Before PR creation, merge, or readiness claims, report:

- reviewed scope and baseline;
- lenses used and any conditional lenses added;
- issues fixed, issues defended, and remaining risk;
- the three consecutive clean iterations;
- local verification commands and outcomes.
