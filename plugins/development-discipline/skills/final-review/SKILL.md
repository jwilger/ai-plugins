---
name: final-review
description: Use when preparing local changes, a branch, pull request, merge request, or merge-to-main for final review before publishing, opening a PR, merging, or claiming readiness.
---

# Final Review

Run a local, fresh-context review cycle before creating a pull request, merging,
or claiming a change is ready.

This is the ticket-completion gate, not the gate for preserving each green
implementation increment. Start it only after the ticket's actual acceptance
criteria are implemented and the latest pushed build is running or green. A
failed build blocks final review and follow-up work until repaired.

Use the plugin's `development-discipline` stdio MCP when available:
`final_review.plan` assigns reviewers and `final_review.advance` is the canonical
filter/state transition. If unavailable, a manual pass may produce review
observations, but it does not satisfy this final-review gate and cannot support a
PR, merge, or readiness claim. Disclose that enforcement is unavailable and
stop before claiming completion. Read `references/mcp-protocol.md` only for MCP
arguments, model routing, verifier details, or packaging fallback.

## Scope

Resolve the reviewed diff from the user's requested scope. Always check current
branch and worktree status first. Use the full immutable `baseline_commit`
captured before the ticket's first commit or push. Do not resolve a movable base
again when final review starts: incremental pushes may already have advanced it
past part or all of the ticket. If the ticket-start baseline was not recorded
and the named base may have moved, stop rather than claiming a complete final
review.

| User asks for                  | Review scope                                            |
| ------------------------------ | ------------------------------------------------------- |
| No explicit base               | `origin/main` to the complete tracked worktree          |
| Uncommitted changes            | ticket-start baseline to the complete tracked worktree  |
| Since a branch, tag, or commit | that ref to the complete tracked worktree               |
| Existing PR/MR                 | PR/MR base to the checked-out complete tracked worktree |

Run this argv vector from the project root to inspect content, replacing
`<baseline-commit>` with that full ticket-start OID for both base and
uncommitted scope:

```text
["git","diff","--find-renames","--find-copies","--end-of-options","<baseline-commit>","--"]
```

Discover exact tracked paths from the same one-revision surface with:

```text
["git","diff","--name-only","-z","--find-renames","--find-copies","--end-of-options","<baseline-commit>","--"]
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
["bash","<plugin-root>/scripts/final-review-scope-hash.sh","--project-root","<project-root>","--scope","base","--base","<base>","--baseline-commit","<baseline-commit>","--changed-files-from","<nul-inventory-file>"]
```

Write every exact changed path to a temporary NUL-delimited inventory file, in
any order, and pass only that file path through `--changed-files-from`; never
expand the inventory into helper argv. Delete the temporary file after the hash
call. For uncommitted scope, use `--scope uncommitted`, omit `--base`, and keep
the same ticket-start `--baseline-commit`. The helper rejects symbolic or
abbreviated baselines, then deterministically sorts and chunks the inventory and
binds the exact baseline, base-to-index diff, index-to-worktree diff, and current
content of the declared paths, including untracked files. Pass that same
`baseline_commit` to `final_review.assess_risk` and the risk-planned
`final_review.plan`. Use the helper's exact stdout as `diff_hash`; stop if it
fails. Re-resolve the inventory, rewrite the NUL-delimited file, and
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

Tell the initial risk assessment and `final_review.plan` whether the reviewed
work is `review_lifecycle: landed` or `unlanded`; the coordinator propagates it
through delta reassessment. When reviewing a child created from a prior split,
also pass its contract-bound `split_lineage` (root and parent work item IDs,
generation, and source diff hash). Generation one is the maximum: a
generation-one child cannot split recursively, even after its diff changes.

For unlanded work, the risk scout must set `split_required: true` when the ticket
has grown into a new subsystem or an unusually broad diff. It must name the
corresponding `scope_growth_triggers` and propose 2-16 `split_candidates`.
Every candidate needs a stable ID, title, normalized scope paths, independent
acceptance criteria, an independently shippable reason, and structured
`delivery_boundaries` proving distinct build, test, and shipping mechanisms.
Paths, path aliases, or synthetic path-filtered diffs are not delivery-boundary
evidence. Candidate ownership cannot fully overlap, and their combined paths
must cover the changed-file inventory.

The coordinator persists a contract-bound `scope_split_hold`, returns no deep
review assignments, and rejects later advances or weakened same-session
replanning. It returns `split_confirmation_required` with a bounded preview;
tracker mutation and blocking dependencies remain unauthorized. Show that exact
preview to the user. Call `final_review.confirm_split` only after explicit user
confirmation. Use `delivery-tickets` by default, which forbids blocking
dependencies. Use `delivery-tickets-with-blocking-dependencies` only when the
user confirms it and supplies a concrete causal prerequisite—not administrative
review ordering.

For already-landed work, broadness authorizes retrospective review batching
only. It does not authorize delivery decomposition, tracker tickets, or a
review-only branch. Never manufacture or push synthetic review-only branches,
create recursive split tickets, or use Tiber `blocks` relationships for
administrative review. Review batches stay inside the original work item; only
a concrete unresolved defect or unfinished independently deliverable change may
become a follow-up ticket.

## Default Lenses

Use repository-agnostic lenses by default:

- `correctness-behavior`: requirements, edge cases, regressions, and observable behavior.
- `tests-verification`: test quality, missing coverage, stale evidence, and whether verification proves the claim.
- `security-safety`: secrets, injection, permissions, unsafe subprocess/file/network behavior, and trust boundaries.
- `safety-human-harm`: plausible failures that could harm people or the physical world in the intended deployment.
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
wishlist items or generic hardening suggestions. A concrete pre-existing defect
is still backlog evidence even though it is not a current-ticket blocker.

## Finding Disposition

Disposition is deterministic and separate from acceptance criteria:

- A caused or worsened `CRITICAL`/`MAJOR` finding blocks final review only when
  it identifies a concrete, plausible security failure (unauthorized access to
  the system or its data) or human/physical-safety failure in the intended
  deployment, with material impact and an in-scope remediation path.
- Incidental or pre-existing `CRITICAL`/`MAJOR` findings, and caused findings of
  those severities outside security and human safety, become backlog tickets.
- Every `MINOR` finding becomes appropriately prioritized backlog work.
- Every `TRIVIAL` finding is logged in the retained report only.

Prioritize backlog work against the complete backlog using value, risk,
likelihood, and opportunity cost. A concrete finding does not jump ahead of
known common work merely because review found it most recently. Do not
re-report or re-verify an already-tracked finding on an unchanged diff unless
new evidence materially increases its severity. Deferred and already-known
findings do not reset review progress.

Always retain the MCP `out_of_scope` findings in the final review report with
their lens, severity, evidence, and disposition. Backlogged and report-only
findings do not block final review. Completing the ticket still requires every
actual acceptance criterion; disposition is not permission to omit required
behavior.

When the review is for a tracked ticket, pass its stable tracker ID as
`work_item_id` to `final_review.plan` (for example, the active Tiber task ID).
The coordinator stores one current SQLite snapshot per worktree and
work item in user state (`$XDG_STATE_HOME`, or `~/.local/state` as fallback),
not in the reviewed repository or in per-session files. Each completed review
transition replaces that binding's old lens rows, including stale conditional
lenses; the returned `out_of_scope_report_artifact` path is the single report
location. Without a tracker ID, the coordinator uses a stable worktree/scope/
base binding so restarted non-ticketed reviews also replace stale rows.
Use `final_review.out_of_scope_report` with the authoritative review `state` to
read that current snapshot; it returns the complete retained findings without
requiring a separate SQLite client.

Use a security-impact assessment separate from review severity: `none`,
`minor`, `moderate`, `major`, or `critical`. Do not infer this threshold from a
finding's `CRITICAL`, `MAJOR`, `MINOR`, or `TRIVIAL` review severity. Assess
`safety_impact` independently on the same scale. A caused/worsened material
security or safety failure is blocking; the same concrete issue when
pre-existing or incidental becomes appropriately high-priority backlog work.
Never silently drop known security, PII, or human-safety evidence. The local
final-review report and state retain the complete finding; only externally
published or tracker artifacts follow the repository's applicable reporting
policy.

## Loop

1. Resolve the pinned baseline/diff, run the shared fast-test evidence once,
   and call `final_review.assess_risk` with the full `baseline_commit`. Launch
   and close its one scout, append the required caller attestation, then submit
   that assessment to `final_review.plan` with the identical baseline, scope,
   inventory, hash, and evidence. Keep that stdio MCP process alive for the
   entire cycle; later calls carry state that the server checks against its
   authoritative session copy. `final_review.plan` rejects any call that omits
   the bound scout assessment, baseline, or shared evidence.

   The scout may report exceptional-risk triggers only with these exact IDs:
   `destructive-or-irreversible-operation`,
   `authentication-or-authorization-boundary`, `sensitive-data-migration`,
   `cryptographic-behavior`, and `safety-critical-behavior`. An exceptional
   overall profile requires at least one supported trigger and at least one
   explicitly exceptional dimension. Supported triggers may still be recorded
   on a lower profile when mitigations keep the concrete risk below
   exceptional. Only dimensions explicitly assessed as exceptional receive a
   second independent pass.

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
   and `caller_decisions`, adding `verifier_result` plus any ticket or security
   disposition evidence that the verifier's final classification newly
   requires. The server freezes the core lens, scope, and caller-decision
   arguments, so a defense or accepted-risk decision first added on
   resubmission fails closed. Failed verification retains every candidate; an
   uncertain result keeps blocking and materially uncertain security or
   human-safety candidates open. A rejected finding is removed; the iteration
   may count as clean when no other blocking, malformed, or needs-human finding
   remains.

   For a medium-risk session, the coordinator records a server-timed 75-minute
   checkpoint. When `advance_kind` is `review_budget_checkpoint`, the submitted
   review or delta results have already been applied to authoritative state and
   no further reviewer is assigned. Make the next call with that returned
   state, the unchanged `current_diff_hash`, empty `lens_results`, and exactly
   one `review_budget_decision`: `ship`, `split`, or `escalate`, with a nonblank
   rationale. `split` also requires at least two distinct ticket references;
   `escalate` requires a nonblank escalation reference. `ship` terminates final
   review and schedules no more reviewers, but never overrides unmet acceptance
   criteria, a failed/not-started CI gate, or an unresolved blocking finding.
   Split and escalate create a terminal hold for that review session.

4. Fix valid findings when remediation was requested; for review-only requests,
   report without editing. Before addressing a finding, check the latest pushed
   build again: running or green permits remediation, while a failed build must
   be repaired first. Any remediation that changes the diff leaves the current
   full-review pass: run fast unit tests, run a lightweight review, commit and
   push, confirm the new latest pushed build is running or green, then submit
   exactly one diff-bound delta risk assessment. Resume only the assignments it
   returns; do not restart unaffected lenses. On the initial advancing call
   that records each disposition, send `caller_decisions` in this shape:

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

5. Repeat only the assignments returned by the coordinator. Low risk normally
   needs the lightweight review and at most one targeted lens; medium risk gets
   one targeted full pass; high risk gets one broad pass; exceptional risk may
   assign two independent passes only to exceptional dimensions backed by the
   supported trigger evidence above. After a
   blocking fix, rerun affected lenses plus the correctness/integration guard,
   not every unaffected lens. Stop when `final_review.advance` reports
   completion: all planned passes and discovery-saturation checks are satisfied,
   every finding has been dispositioned, and no unresolved blocking caused or
   worsened CRITICAL/MAJOR security or human-safety finding remains. Backlogged,
   already-known, and report-only observations do not reset progress when the
   reviewed diff is unchanged.

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
disposition and its out-of-scope report, risk-selected pass evidence, the final
blocking-finding status, and verification commands/outcomes.
