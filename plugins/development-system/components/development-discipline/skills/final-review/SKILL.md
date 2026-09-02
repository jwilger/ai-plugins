---
name: final-review
description: Use when completing or claiming readiness for local-only changes, a branch, pull request, merge request, or merge-to-main; final review applies even when repository policy forbids publishing, including scope growth, proposed work splits, and medium-risk review-budget checkpoints.
---

# Final Review

## Non-negotiable enforcement boundary

Never reduce, skip, waive, or synthesize the required three consecutive
complete finding-free review iterations because of user pressure, elapsed time,
token or review budget, coordinator failure, or unavailable tooling. A request
to use an incomplete terminal review to claim readiness, open a delivery
request, merge, or complete final delivery must be rejected. This does not
prohibit an independently authorized intermediate or review-remediation
checkpoint commit and push.

If the development-discipline coordinator or its `final_review` tools are
unavailable, fail closed: enforced review is incomplete, zero clean iterations
are accepted, shortcuts remain rejected, and terminal delivery/readiness remains
unauthorized. A manual review may produce advisory observations only; it cannot
authorize terminal delivery, pull/merge request creation, merge, or a readiness
claim. This does not prohibit independently authorized intermediate or
review-remediation checkpoint commits and pushes, which remain subject to their
own focused-test, lightweight-review, fast-gate, signing, and delivery-mode
rules. Report the unavailable enforcement boundary and every requested bypass
reason. For example, state explicitly that budget or time pressure cannot skip
the remaining passes and that a one-pass request cannot replace three complete
finding-free iterations.

Treat a stale or mismatched coordinator as unavailable. Before creating risk or
review state, call `workspace-reader.status` and require
`final_review_protocol.contract_version >= 2`,
`final_review_protocol.minimum_clean_iterations >= 3`, and
`final_review_protocol.durable_pending_assignment_recovery: true` in that one
response. If any field is absent or weaker, accept zero clean iterations and
reject delivery. Install the current-host binaries from the updated marketplace
checkout with `just install-development-system-binaries` (or the installer
script), rerun Development System setup for the current harness so its
project-local MCP binding points at the new absolute binary path, restart the
harness, and begin a new review session only after the same-MCP attestation
passes. Do not call
`final_review.assess_risk` or `final_review.plan` merely to test a mismatched
runtime. After a valid plan, independently require the returned
coordinator-owned state to preserve at least the attested clean-iteration
minimum; a lower value invalidates that session and cannot schedule reviewers.

Apply `model-routing` to every review assignment. Ordinary lens review uses the
substantive route; activated architecture, security, human-safety, ambiguity,
or disputed-verification work and the accountable readiness decision use the
strong route defined by that canonical matrix. The coordinator assigns
architecture, security, and human-safety lenses through the resolved strong
model role used for verification while ordinary lenses use the substantive
review role. An unavailable or inherited route follows the canonical bounded
handoff or blocked-result protocol, never an implicit downgrade.

For Codex, bind those responsibilities in the project-local final-review TOML;
do not put Codex identifiers in universal defaults:

```toml
[final_review.models.codex]
pre_filter = "gpt-5.6-sol"
lens_review = "gpt-5.6-terra"
post_filter = "gpt-5.6-luna"
verifier = "gpt-5.6-sol"
```

`pre_filter` owns the all-dimension broad risk scout. For Codex, `lens_review`
is Terra for ordinary risk-selected substantive lenses. The coordinator routes
an assigned architecture, security, or human-safety lens through the resolved
strong role: `gpt-5.6-sol`. `post_filter` labels
the normally deterministic
`final_review.filter_findings` path; deterministic relevance and path filtering
normally make no model call. `verifier` owns blocking, disputed, or materially
uncertain batched verification. These roles are part of the review contract,
not permission for the MCP to spawn agents. The caller starts every assignment
as a fresh-context subagent, closes it immediately after receiving the result,
and submits the required caller attestation naming the assigned model role plus
`fresh_context: true` and `closed_after_result: true`.

Run a local, fresh-context review cycle before creating a pull request, merging,
or claiming a change is ready.

## Delivered-checkpoint terminal boundary

Final review starts only after every planned implementation increment and every
actual acceptance criterion have been completed and checkpoint-delivered. It
consumes that ticket-complete identity: the exact pushed commit for
direct-to-trunk or PR/MR mode, or the exact local snapshot for local-only. A
clean unchanged review adds review evidence only; it creates no commit, empty
commit, push, or replacement local checkpoint. In remote modes, required pushed
CI may run concurrently with review, but readiness requires terminal success
for the exact final-reviewed pushed SHA. In local-only mode, readiness instead
requires fresh evidence bound to the exact final-reviewed local identity and
never requires or invents remote CI.

When explaining this boundary, explicitly state that every remote remediation
checkpoint verifies the exact commit, commit message, and signature, and that a
material delta reruns the complete selected lens set in fresh contexts. Do not
leave either requirement implied by generic checkpoint or reset language.

Only the pinned Git baseline and the in-repository changed-file inventory define
the reviewed source scope and its hash. Coordinator EventCore facts, snapshots,
state files, and projections are bookkeeping outside that inventory. Their
creation or update cannot reopen source review or enter the reviewed-content
hash.

Retain the delivered identity's pinned ticket-start baseline, requested scope,
exact path inventory, and each path's reviewed content and mode. A path
addition, removal, rename, mode change, content change, newly in-scope untracked
file, different baseline, or different requested scope is source-changing
remediation: create its new mode-specific checkpoint first, then perform the
delta assessment and complete clean-iteration reset against that new identity.

During an active review, continue to rerun the bundled stage-aware scope hash
before every advance and treat a changed hash as the protocol requires. Never
use the delivery boundary to excuse a staged, unstaged, mode, path, or untracked
content change while review is active.

Each remote checkpoint commit already carries exact-commit verification of the
required fast non-duplicated checks plus message and signature. Comprehensive
remote evidence remains in CI. A clean terminal review creates no post-review
commit. In remote modes, only terminal-success CI for the exact final-reviewed
pushed SHA supports readiness; pending CI yields
`review-complete-awaiting-exact-sha-ci`. In local-only mode, fresh readiness
evidence must be bound to the exact final-reviewed local identity; remote CI is
not a gate and must not be requested.

This is the ticket-completion gate, not the gate for preserving each green
implementation increment. Start it only after every planned implementation
increment and every actual acceptance criterion are completed and
checkpoint-delivered and no prior failed-run hold remains. Direct-to-trunk and
PR/MR review consumes the exact already-pushed final checkpoint SHA; PR creation
and merge remain separately authorized operations. Required exact-SHA CI may be
pending while remote-mode review proceeds, but any completed failure activates
`ci-failure-follow-up` immediately. A clean remote-mode review with pending CI
reports `review-complete-awaiting-exact-sha-ci`; readiness waits for terminal
success on the exact final-reviewed pushed SHA. Local-only review consumes the
exact local identity, requires fresh readiness evidence bound to that identity,
and never creates a remote action or waits for remote CI solely to unlock
review.

A failed pushed build invokes `ci-failure-follow-up` and blocks final review and
follow-up work until that skill's terminal-success hold is released; a newer
running build does not mask an earlier hold or disappear in another mode.

Use the plugin's `development-discipline` stdio MCP when available:
`final_review.plan` assigns reviewers and returns a compact `state_ref` for
subsequent calls. `final_review.advance` is the canonical filter/state
transition when the plan has assignments. A plan with `assignments: []` and
`complete: true` is already terminal; do not call `final_review.advance`. On the
plugin's advisory surface, stop there without calling a native workflow tool.
Standalone Tiber may instead return `next_tool: workflow.record_clean_review`;
when that native workflow service is actually available, pass the plan's
`state_ref` to it as `review_state_ref`. If the MCP
is unavailable, a manual pass may produce review
observations, but it does not satisfy this final-review gate and cannot support a
PR, merge, or readiness claim. Disclose that enforcement is unavailable and
stop before claiming completion. Read `references/mcp-protocol.md` only for MCP
arguments, model routing, verifier details, or packaging fallback.

## Scope-Growth Guardrail

Tell the initial risk assessment and `final_review.plan` whether the reviewed
work is `review_lifecycle: landed` or `unlanded`; the coordinator propagates it
through delta reassessment. When reviewing a child created from a prior split,
also pass its contract-bound `split_lineage` (root and parent work item IDs,
generation, and source diff hash). Generation one is the maximum: a
generation-one child cannot split recursively, even after its diff changes.

For unlanded work, the risk scout must set `split_required: true` when the ticket
meets either predicate below:

- `new-subsystem`: scope added beyond the original request or acceptance
  criteria crosses a runtime, ownership, or delivery boundary and can ship with
  its own acceptance criteria plus build, test, and shipping mechanisms.
- `unusually-broad-diff`: the work contains at least two internally cohesive,
  low-coupling increments that can be accepted and shipped independently.
  File count, path count, generated churn, or diff size alone never satisfies
  this predicate.

If the scout cannot construct valid independently shippable candidates, it must
not assert either trigger merely because review is large; use risk-selected
review batching instead. When a predicate is met, name the corresponding
`scope_growth_triggers`, give a nonblank split rationale, and propose 2-16
`split_candidates`.
Every candidate needs a stable ID, title, normalized scope paths, independent
acceptance criteria, an independently shippable reason, and structured
`delivery_boundaries` proving distinct build, test, and shipping mechanisms.
Paths, path aliases, or synthetic path-filtered diffs are not delivery-boundary
evidence. Candidate ownership cannot fully overlap, and their combined paths
must cover the changed-file inventory. A candidate is cohesive when its paths
serve one acceptance contract; candidates are low-coupling when one can build,
test, and ship without the other's unfinished behavior.

The coordinator persists a contract-bound `scope_split_hold`. The hold means
exactly: it returns no assignments, the review remains incomplete, and no later
advance or weakened same-session replan can bypass confirmation. It
returns
`split_confirmation_required` with a bounded preview; tracker mutation and
blocking dependencies remain unauthorized. Show that exact preview to the
user. Call `final_review.confirm_split` only after explicit user confirmation;
standing execution approval never confirms a split.

These invariants are non-bypassable:

- Reject weakened same-session replanning while the hold is active.
- Never infer split confirmation from standing execution approval.
- Reject every recursive child split, even after the child's diff changes.
- Reject a risk plan that selects no deep-review lens.
- Require every selected lens, plus every assigned verifier, in every complete
  review iteration. Inside the authoritative `final_review.advance` transition,
  normalize schema-invalid lens evidence, caller-attestation-invalid lens
  evidence, and malformed, provenance-invalid, or coverage-invalid verifier
  evidence as bounded non-clean results. Each case restarts the complete
  selected-lens set instead of carrying peer evidence into a later iteration.
- In the source-level standalone Tiber review contract, a current result that
  fails scheduler provenance or finding-identity checks emits durable
  `AssignmentResultRejected`, invalidates that full iteration, and permits only
  fresh next-iteration assignments. That contract is not yet bound to the
  installed Tiber task-board binary.
- Require at least three consecutive complete finding-free iterations. Any
  reported finding resets both the clean streak and its durable verified-clean
  evidence, even when the finding is later rejected, defended, accepted,
  routed, or documented as already tracked.
- An out-of-scope wishlist finding is report-only: do not implement or backlog
  it. Its finding-bearing iteration is still non-clean, so restart the complete
  risk-selected lens set instead of preserving peer evidence. Separately verify
  the ticket's actual acceptance criteria, and require `final_review.advance`
  to report three later consecutive complete finding-free iterations with no
  unresolved blocking caused or worsened CRITICAL/MAJOR security or
  human-safety finding.
- Preserve that minimum across risk planning, delta reassessment, verifier
  continuation, review-budget decisions, and completion checks. No timeout,
  `ship` choice, caller-carried state, or risk-selected pass count may lower it.

Use `delivery-tickets` by default, which forbids blocking dependencies. Use
`delivery-tickets-with-blocking-dependencies` only when the user confirms it
and supplies a concrete causal prerequisite—not administrative review ordering.

For already-landed work, broadness authorizes retrospective review batching
only. It does not authorize delivery decomposition, tracker tickets, or a
review-only branch. Never manufacture or push synthetic review-only branches,
create recursive split tickets, or use Tiber `blocks` relationships for
administrative review. Review batches stay inside the original work item; only
a concrete unresolved defect or unfinished independently deliverable change may
become a follow-up ticket.

## Medium-Risk Review Budget

For a medium-risk session, the coordinator records a server-timed 75-minute
checkpoint. Apply this contract when `advance_kind` is
`review_budget_checkpoint`:

- The coordinator has already persisted the submitted review or delta findings
  to authoritative state and returned no further reviewer assignments.
- Make the next call with the returned state, unchanged `current_diff_hash`,
  empty `lens_results`, and exactly one `review_budget_decision` with a nonblank
  rationale.
- For unlanded reviews, the choices are `ship`, `split`, or `escalate`; `split`
  requires at least two distinct ticket references. For landed reviews, the
  choices are only `ship` or `escalate` because landed work cannot be decomposed
  into delivery tickets. `escalate` requires a nonblank escalation reference.
- Reject `ship` until every planned increment and acceptance criterion is
  delivered, every blocking finding is resolved, and the durable review state
  contains at least three consecutive complete finding-free iterations. For
  direct-to-trunk or PR/MR mode, also require terminally successful CI for the
  exact final-reviewed pushed SHA with no current completed failed job. For
  local-only mode, instead require fresh readiness evidence bound to the exact
  final-reviewed local identity and require no remote CI. Once valid, `ship` is
  terminal and schedules no more reviewers.
- For unlanded reviews, `split` creates a terminal hold. `escalate` creates one
  in either lifecycle. Each hold preserves blockers, schedules no reviewers,
  and rejects every later `final_review.advance` for that session.

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

## Default Lenses

Use repository-agnostic lenses by default:

- `correctness-behavior`: requirements, edge cases, regressions, and observable behavior.
- `tests-verification`: test quality, missing coverage, stale evidence, and
  whether verification proves the claim. Require a new RED test only for new or
  changed first-party production behavior without a clear existing failure. Do
  not demand one for non-code changes, removals, third-party behavior, committed
  static text, straightforward CI scripting, simple non-production developer
  utilities, or behavior-preserving refactors with adequate green coverage.
  Flag tests that restate documented third-party APIs or examples; remove them
  when no application contract is at stake, or replace them with
  dependency-agnostic black-box coverage of the application's observable
  integration behavior.
  Treat tests that only inspect committed repository text or CI workflow
  definitions as actionable findings; require removal or replacement with
  public observable behavior. For files a
  program creates or edits, prefer the end-user-visible effect and accept exact
  generated-text assertions only when no behavioral test can prove the
  requirement. Inspect the surrounding project test scope for existing
  instances of these anti-patterns and report them even when they predate the
  diff. Recommend removal, public-behavior replacement, or extraction of an
  overgrown utility into a maintained project or shipped subsystem. Preserve
  valuable failure-mode coverage until the extraction carries the behavior and
  tests with it.
  For removals, require evidence that production functionality changed before
  its tests, the unchanged suite exposed affected expectations, obsolete tests
  were then deleted or updated, and retained behavior returned to green.
  Reviewing only the lines in the proposed diff is incomplete; perform the
  surrounding audit and act on what it finds without requiring ritual policy
  restatement when the findings and disposition already make the action clear.
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

Always select `production-risk-footguns` as an independent final-review lens.
Other lenses remain risk-selected; this one is mandatory because latent traps,
scale-dependent data/resource access, and burst behavior are not reliably
covered by correctness or operability review.

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

Every actionable finding must also state whether it is `caused`, `worsened`,
`pre-existing`, or `incidental`; cite the changed path/symbol or matched review
context; and name the failure mechanism, required precondition, affected
behavior or asset, intended deployment, and impact. Security findings must name
the in-model actor or untrusted input, crossed trust boundary, affected asset,
and unauthorized outcome. Human-safety findings must name the initiating
failure, hazard, exposure path, and plausible consequence. A generic hardening
preference or an actor outside the repository's threat model is not a failure
path.

A user-request, acceptance-criteria, or explicit-concern finding may cite an
unmodified path when it includes `matched_context` copied exactly from the
supplied review context. Other unmodified-file or nearby-context findings need a
causal path from this change; otherwise they are out of scope. A reviewer that
reports any canonical finding still makes that iteration non-clean; disposition
controls follow-up work, not whether the pass was finding-free. A real
challenge to a prior defense is non-clean until
resolved and accepted by a later relevant-lens review. Generic best practice or
a hypothetical improvement is not a cross-cutting risk without a concrete
failure path caused by the current change. Do not fix or backlog out-of-scope
wishlist items or generic hardening suggestions. A concrete pre-existing defect
is still backlog evidence even though it is not a current-ticket blocker.

Filtering an out-of-scope finding does not change the risk-selected review plan
and does not waive any acceptance criterion. Continue the assigned passes and
separately verify every actual acceptance criterion. Completion still requires
`final_review.advance` to report completion with no unresolved blocking caused
or worsened CRITICAL/MAJOR security or human-safety finding.

When explaining an out-of-scope disposition, explicitly state every invariant:
the finding-bearing iteration is non-clean; restart the complete risk-selected
lens set instead of preserving unaffected evidence; separately verify the
ticket's actual acceptance criteria; and require `final_review.advance` to
report three later consecutive complete finding-free iterations with no
unresolved blocking caused or worsened CRITICAL/MAJOR security or human-safety
finding.

## Finding Disposition

Disposition is deterministic and separate from acceptance criteria:

- A caused or worsened `CRITICAL`/`MAJOR` finding blocks final review only when
  it satisfies the security or human-safety evidence contract above, has
  `security_impact` or `safety_impact` of `major` or `critical`, and names an
  in-scope changed remediation path. The word `material` is not an additional
  free-form threshold.
- Incidental or pre-existing `CRITICAL`/`MAJOR` findings, and caused findings of
  those severities outside security and human safety, become backlog tickets.
- Every `MINOR` finding becomes appropriately prioritized backlog work.
- Every `TRIVIAL` finding is logged in the retained report only.

Prioritize backlog work against the complete backlog using value, risk,
likelihood, and opportunity cost. A concrete finding does not jump ahead of
known common work merely because review found it most recently. Do not
re-report or re-verify an already-tracked finding on an unchanged diff unless
new evidence materially increases its severity. Deferred and already-known
findings do not reset progress merely because they remain in durable history.
If a reviewer nevertheless submits one again as a canonical finding, the
current iteration is non-clean and the complete selected lens set restarts.

Always retain the MCP `out_of_scope` findings in the final review report with
their lens, severity, evidence, and disposition. Backlogged and report-only
findings do not block final review. Completing the ticket still requires every
actual acceptance criterion; disposition is not permission to omit required
behavior.

When the review is for a tracked ticket, pass its stable tracker ID as
`work_item_id` to `final_review.plan` (for example, the active Tiber task ID).
The coordinator stores decisions as EventCore facts. The advisory plugin uses a
separate local-only Git authority under repository Git metadata and never
publishes those review facts to a remote; standalone Tiber's native workflow
service uses its configured Development Workflow authority. One rebuildable
SQLite report/projection per worktree and work item lives in user state
(`$XDG_STATE_HOME`, or `~/.local/state` as fallback), not in the worktree or in
per-session files. Each completed review transition replaces that binding's old
lens rows, including stale conditional lenses; the returned
`out_of_scope_report_artifact` path is the single report location. Without a
tracker ID, the coordinator uses a stable worktree/scope/base binding so
restarted non-ticketed reviews also replace stale rows.
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
   inventory, hash, and evidence. Later calls should carry the returned compact
   `state_ref`, which the server resolves against its durable authoritative
   session copy; the full `state` remains a compatibility form and must never be
   summarized or edited. Keep the stdio MCP process alive when practical. After
   an ordinary process restart or a lost handoff, call
   `final_review.resume_latest` with `session_id`, `project_root`, and optional
   `work_item_id` to obtain the current `state_ref` and compact pending
   assignments without advancing the review. If a plan response is too large or
   truncated, call `final_review.pending_assignments` with that `state_ref`;
   never reconstruct assignment keys, roles, or schemas from memory. The
   summary is read-only and stable. Pass one exact `subagent_key` back to the
   same tool only when its full prompt and result schema are needed.
   `final_review.plan` rejects any call that omits
   the bound scout assessment, baseline, or shared evidence.

   The scout always assesses every assigned dimension. A low overall profile
   may still report several concrete low or uncertain dimensions; the
   coordinator deterministically schedules at most one targeted lens instead
   of rejecting the complete assessment. If `final_review.plan` reports
   `risk_assessment_identity_mismatch`, compare its sanitized expected and
   received assignment fingerprints. Resume the matching assessment when it is
   still available; otherwise rerun `final_review.assess_risk` with the exact
   intended plan contract, launch its replacement scout, and resubmit that new
   assessment unchanged. Abandon a stale assessment instead of editing its
   identity fields. The diagnostic names the mismatched field and fingerprints
   but never exposes either raw identity.

   The scout may report exceptional-risk triggers only with these exact IDs:
   `destructive-or-irreversible-operation`,
   `authentication-or-authorization-boundary`, `sensitive-data-migration`,
   `cryptographic-behavior`, and `safety-critical-behavior`. An exceptional
   overall profile requires at least one supported trigger and at least one
   explicitly exceptional dimension. Supported triggers may still be recorded
   on a lower profile when mitigations keep the concrete risk below
   exceptional. Only dimensions explicitly assessed as exceptional receive a
   second independent discovery pass. Do not confuse that dimension-scoped
   discovery sample with the clean-review iterations. Revalidate every later
   delta by these same supported-trigger and explicitly-exceptional-dimension
   rules. If a valid delta legitimately raises only safety to exceptional,
   merge and retain its supported trigger evidence in authoritative state, give
   only safety the additional discovery sample, clear the clean streak and
   verified receipts, invalidate all old peer results, and then rerun every
   selected lens for three fresh consecutive complete finding-free iterations.

2. For every assignment, use the compact summary's exact lens, iteration,
   `subagent_key`, `model_role`, close policy, shared-evidence ID, and schema
   version. Retrieve that assignment's full prompt individually, then start a
   fresh subagent with the complete MCP-generated
   assignment prompt, including its baseline, diff, relevant files, user
   request, acceptance criteria, explicit concerns, and prior defenses. Exclude
   unrelated conversation context. Return the assigned schema and exact
   `subagent_key`; close the subagent immediately, then append
   `caller_attestation` with its assigned model role, `fresh_context: true`, and
   `closed_after_result: true`. Carry continuity only through MCP state,
   defenses, and caller decisions.
   A missing or invalid lifecycle/model attestation is malformed assigned
   evidence. Accept the coordinator's returned non-clean reset transition,
   discard every result from that iteration, and run the complete selected lens
   set from its fresh next-iteration assignments. Never repair and resubmit the
   old assignment or preserve peer results from the invalidated iteration.
3. If the plan is already complete with no assignments, stop this loop. On the
   plugin's advisory surface, do not call a native workflow handoff. In
   standalone Tiber, when the response names
   `next_tool: workflow.record_clean_review` and that native service is
   available, pass its `state_ref` as `review_state_ref`. Otherwise call
   `final_review.filter_findings` with the returned `state_ref` and complete
   `lens_results`. Prepare any applicable `caller_decisions` from its retained
   findings before the first `final_review.advance` call; include those
   decisions on that initial call, which may return `verifier_required`.
   Re-resolve the complete changed-file inventory and rerun the bundled scope
   hash helper. Call `final_review.advance` with the returned `state_ref`, all
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
   human-safety candidates open. A rejected finding is removed from the
   unresolved candidate set, but the original finding-bearing iteration remains
   non-clean and resets all clean-streak credit. Run every returned fresh lens
   assignment; only three later complete finding-free iterations may complete
   review. A malformed verifier result or one with invalid assignment
   provenance, lifecycle/model attestation, or verdict coverage is instead
   consumed as a non-clean reset transition: discard the whole iteration and
   run every returned fresh lens assignment.

4. Fix valid findings when remediation was requested; for review-only requests,
   report without editing. Any completed required CI failure must follow
   `ci-failure-follow-up` first and preempts remediation or resumed review.
   Otherwise, remediation that changes the diff leaves the current full-review
   pass: classify whether RED applies, use it when required, run the immediate
   focused test, lightweight review, and repository fast checkpoint gate.
   Direct-to-trunk uses a signed additive commit, exact commit/message/signature
   verification, and normal push. PR/MR uses a signed additive commit and only
   an already-authorized branch push without inferring PR operations. Local-only
   creates a signed local remediation commit only when that commit is authorized;
   otherwise it records a new exact no-commit snapshot and never pushes to
   resume review. If repository policy requires committed evidence but the user
   explicitly withholds commit authority, stop as blocked without committing or
   pushing; do not silently replace the required evidence with a snapshot.
   Submit exactly one diff-bound delta risk assessment and run the complete
   selected lens set in fresh contexts. In remote modes, required exact-SHA CI
   may remain pending; a later completed failure immediately preempts through
   Tiber recovery. In local-only mode, refresh readiness evidence against the
   new exact local identity and do not request remote CI. The material delta
   invalidates the old iteration and no unaffected peer evidence carries
   forward. On the initial advancing call
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
   assignments. The coordinator must give each defense or caller decision back
   to its relevant fresh reviewers. The finding-bearing iteration remains
   non-clean; only three later consecutive complete finding-free iterations may
   complete review.

5. Repeat every assignment returned by the coordinator. Risk determines the
   selected lens set and whether exceptional dimensions need an additional
   independent discovery sample; it never lowers the three-iteration clean
   minimum. Every clean iteration reruns the complete selected lens set. A
   malformed result, any canonical finding, or a material diff change resets
   the clean streak; after a fix or delta, rerun every selected lens. Stop only
   when `final_review.advance` reports completion: discovery-saturation checks
   are satisfied, three consecutive complete iterations reported no findings,
   every prior finding has been dispositioned, and no unresolved blocking
   caused or worsened CRITICAL/MAJOR security or human-safety finding remains.

This skill requires a harness that can launch fresh-context subagents. If that
capability is unavailable, stop and report that final-review cannot be
completed to this standard. The MCP
rejects stale full state or stale state references, enforces result keys/sets,
verifier gates, and terminal completion, validates the caller's explicit
model/fresh-context/shutdown attestations, and binds model routing into the
review contract.

## Output

Before PR creation, merge, or readiness claims, report the scope/baseline,
lenses, fixes/defenses/remaining risk, the selected unrelated-finding
disposition and its out-of-scope report, risk-selected pass evidence, the final
blocking-finding status, and verification commands/outcomes.
