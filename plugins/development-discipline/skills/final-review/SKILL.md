---
name: final-review
description: Use when preparing local changes, a branch, pull request, merge request, or merge-to-main for final review before publishing, opening a PR, merging, or claiming readiness.
---

# Final Review

Run a local, fresh-context review cycle before creating a pull request, merging,
or claiming a change is ready.

Use the plugin's `development-discipline` stdio MCP when available:
`final_review.plan` assigns reviewers and `final_review.advance` is the canonical
filter/state transition. If unavailable, a manual pass may produce review
observations, but it does not satisfy this final-review gate and cannot support a
PR, merge, or readiness claim. Disclose that enforcement is unavailable and
stop before claiming completion. Read `references/mcp-protocol.md` only for MCP
arguments, model routing, verifier details, or packaging fallback.

## Scope

Resolve the reviewed diff from the user's requested scope. Always check current
branch and worktree status first.

| User asks for                  | Review scope                                            |
| ------------------------------ | ------------------------------------------------------- |
| No explicit base               | `origin/main` to the complete tracked worktree          |
| Uncommitted changes            | `HEAD` to the complete tracked worktree                 |
| Since a branch, tag, or commit | that ref to the complete tracked worktree               |
| Existing PR/MR                 | PR/MR base to the checked-out complete tracked worktree |

For base scope, run this argv vector from the project root to inspect content,
replacing `<base>` with the resolved ref; for uncommitted scope, use `HEAD`:

```text
["git","diff","--find-renames","--find-copies","--end-of-options","<base>","--"]
```

Discover exact tracked paths from the same one-revision surface with:

```text
["git","diff","--name-only","-z","--find-renames","--find-copies","--end-of-options","<base>","--"]
```

Parse its NUL-delimited records as exact paths; never derive names from the
human-readable content diff. Also run
`["git","status","--short","-z","--untracked-files=all"]`, parse its
porcelain-v1 `-z` output without display unquoting: each primary record is two
status bytes plus one space followed by the raw path. Remove only that three-byte
prefix. When either status byte is `R` or `C`, consume the following NUL field as
the source path; `-z` emits destination then source and omits `->`. Retain the
actual destination and source paths, never status bytes or separators, and
inspect declared in-scope untracked files directly because Git diff omits their
content. Merge and exact-byte-deduplicate the tracked-diff and status paths to
derive the complete `changed_files` inventory. A clean status does not make base
scope empty when the branch contains committed changes. Resolve `<plugin-root>`
as the development-discipline plugin directory containing this skill, then
derive `diff_hash` only with the bundled helper:

```text
["bash","<plugin-root>/scripts/final-review-scope-hash.sh","--project-root","<project-root>","--scope","base","--base","<base>","--changed-files-from","<nul-inventory-file>"]
```

Write every exact changed path to a temporary NUL-delimited inventory file, in
any order, and pass only that file path through `--changed-files-from`; never
expand the inventory into helper argv. Delete the temporary file after the hash
call. For uncommitted scope, use `--scope uncommitted` and omit `--base`. The
helper deterministically sorts and chunks the inventory, then binds the resolved
base, base-to-index diff, index-to-worktree diff, and current content of the
declared paths, including untracked files. Use its exact stdout as `diff_hash`;
stop if it fails. Re-resolve the inventory, rewrite the NUL-delimited file, and
rerun the helper immediately before every `final_review.advance` call. Do not
substitute a triple-dot, index-only, bare worktree, caller-invented hash, or
path-per-argument invocation; those can omit scope or fail at valid large-scope
sizes.

If the base is ambiguous, infer the safest local scope and state it. Do not
review excluded local dirt; disclose it before any readiness claim. Capture the
changed files/diff hash plus the request, acceptance criteria, explicit concerns,
and prior defenses. When an accepted defense predates this MCP session, include
it in `final_review.plan` as a bounded `prior_defenses` entry with exact `id`,
`lens`, `decision` (`defended` or `accepted-risk`), and a `defense` containing
at least one non-whitespace character.
The MCP binds imported defenses into the initial contract and gives each one to
the matching first-iteration lens. Do not rely on conversation context alone.

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
for UI work or agent-instruction quality for prompt/plugin changes. Give every
conditional lens a concise objective; the MCP rejects identifier-only lenses so
fresh reviewers always receive a distinct review contract.

## Relevance Gate

Lenses define what to inspect, not what the ticket requires. A concern is not
relevant merely because a lens covers it. Do not invent acceptance criteria,
deliverables, infrastructure, CI work, refactors, or follow-up tasks.

Every finding must state its relevance to at least one of:

- the reviewed diff, changed files, or PR scope;
- the user's requested task, acceptance criteria, or explicit concern;
- a prior unresolved review thread or defense that remains contradicted by the
  current diff;
- a real cross-cutting safety, data-loss, security, compatibility, or release
  risk introduced or exposed by the current change.

A user-request, acceptance-criteria, or explicit-concern finding may cite an
unmodified path when it includes `matched_context` copied exactly from the
supplied review context. Other unmodified-file or nearby-context findings need a
causal path from this change; otherwise they are out of scope and do not reset
the clean streak. A real challenge to a prior defense is non-clean until
resolved and accepted by a later relevant-lens review. Generic best practice or
a hypothetical improvement is not a cross-cutting risk without a concrete
failure path caused by the current change. Do not fix or backlog out-of-scope
observations unless the user asks.

## Unrelated-Finding Disposition

At the start of work on one ticket, ask the user once for an unrelated-finding
disposition matrix. It may set `address-now`, `follow-up-ticket`, or `report`
by lens and/or severity class, plus a default for anything unmatched.
For example, architecture notes can be report-only while release warnings
become follow-up tickets. For a multi-ticket automation, ask once before the
automation begins and retain that policy for its duration. Do not ask again for
every review iteration or finding.

Always retain the MCP `out_of_scope` findings in the final review report with
their lens, severity, evidence, and the selected disposition. They do not reset
the clean streak and are not current-ticket blockers unless the user explicitly
chooses to include them. A finding introduced by, caused by, or blocking the
current ticket remains actionable regardless of this preference.

When the review is for a tracked ticket, pass its stable tracker ID as
`work_item_id` to `final_review.plan` (for example, the active Tiber task ID).
The coordinator stores one current, sanitized SQLite snapshot per worktree and
work item in user state (`$XDG_STATE_HOME`, or `~/.local/state` as fallback),
not in the reviewed repository or in per-session files. Each completed review
transition replaces that binding's old lens rows, including stale conditional
lenses; the returned `out_of_scope_report_artifact` path is the single report
location. Without a tracker ID, the coordinator uses a stable worktree/scope/
base binding so restarted non-ticketed reviews also replace stale rows.
Use `final_review.out_of_scope_report` with the authoritative review `state` to
read that current sanitized snapshot; it returns the metadata rows without
requiring a separate SQLite client or exposing reviewer prose.

Use a security-impact assessment separate from review severity: `none`,
`minor`, `moderate`, `major`, or `critical`. Do not infer this threshold from a
finding's `error`, `warning`, or `note` review severity. A major-or-higher
security issue, or any suspected PII exposure at any security-impact level, is
an exception: unless it must be fixed in the current ticket, document it as a
high-priority bug ticket even when the selected disposition is report-only.
Never silently drop, defer without documentation, or let a user opt out of
recording such a finding. Do not include sensitive PII or exploit details in a
general report; use the repository's approved security-reporting path.

## Loop

1. Resolve the baseline/diff and call `final_review.plan`. Keep that stdio MCP
   process alive for the entire cycle; later calls carry state that the server
   checks against its authoritative session copy.
2. For every assignment, start a fresh subagent with the complete MCP-generated
   assignment prompt, including its baseline, diff, relevant files, user
   request, acceptance criteria, explicit concerns, and prior defenses. Exclude
   unrelated conversation context. Return the assigned schema and exact
   `subagent_key`; close the subagent immediately, then append
   `caller_attestation` with its assigned model role, `fresh_context: true`, and
   `closed_after_result: true`. Carry continuity only through MCP state,
   defenses, and caller decisions.
3. Call `final_review.filter_findings` with the returned `state` and complete
   `lens_results`. Prepare any applicable `caller_decisions` from its retained
   findings before the first `final_review.advance` call; include those
   decisions on that initial call, which may return `verifier_required`.
   Re-resolve the complete changed-file inventory and rerun the bundled scope
   hash helper. Call `final_review.advance` with the returned `state`, all
   `lens_results`, and that exact output as `current_diff_hash` on every
   iteration. When it differs from the state's scope hash, also include the
   complete `current_changed_files` inventory. If it returns
   `verifier_required`, run and immediately close that one batched assignment
   and append the same caller attestation. Resubmit the exact same `state`,
   `lens_results`, `current_diff_hash`, any required `current_changed_files`,
   and `caller_decisions`, adding only `verifier_result`. The server freezes
   pre-verifier arguments, so a defense or accepted-risk decision first added
   on resubmission fails closed. Failed/uncertain verification keeps candidates
   open; a rejected finding is not a blocker but its iteration is still
   non-clean.
4. Fix valid findings when remediation was requested; for review-only requests,
   report without editing. On the initial advancing call that records each
   disposition, send `caller_decisions` in this shape:

   ```json
   [
     {
       "finding_id": "<exact id>",
       "lens": "<exact lens>",
       "decision": "defended",
       "defense": "<concise rationale>"
     }
   ]
   ```

   `decision` must be exactly `fixed`, `defended`, or `accepted-risk`. `fixed`
   resolves only after the reviewed diff changes; `defended` and
   `accepted-risk` require a `defense` containing at least one non-whitespace
   character. Do not rely on conversation prose to carry a decision into later
   assignments.

5. Repeat with fresh assignments. Stop only when `final_review.advance` reports
   completion after three consecutive full iterations with no actionable,
   needs-human, malformed, or unresolved finding. A filtered out-of-scope
   observation does not break an otherwise clean iteration. A defense counts as
   clean only after the next relevant lens accepts it.

This skill requires a harness that can launch fresh-context subagents and keep
one MCP process alive through the review. If either capability is unavailable,
stop and report that final-review cannot be completed to this standard. The MCP
rejects stale or mutated caller-carried state, enforces result keys/sets,
verifier gates, and terminal completion, validates the caller's explicit
model/fresh-context/shutdown attestations, and binds model routing into the
review contract.

## Output

Before PR creation, merge, or readiness claims, report the scope/baseline,
lenses, fixes/defenses/remaining risk, the selected unrelated-finding
disposition and its out-of-scope report, three clean iterations, and
verification commands/outcomes.
