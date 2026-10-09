---
name: final-review
description: Use when completing or claiming readiness for local-only changes, a branch, pull request, merge request, or merge-to-main; final review applies even when repository policy forbids publishing, including scope growth, proposed work splits, and medium-risk review-budget checkpoints.
---

# Final Review

## Non-negotiable enforcement boundary

Read [proportional review policy](references/proportional-review-policy.md) before
planning, repairing, migrating, or claiming completion. Protocol v3 requires one
complete risk-selected independent round by default, plus only the justified
additional samples recorded for consequential responsibilities. Completion means
valid scope coverage and resolved blockers, not a caller-maintained clean streak.
Never waive required coverage, security scrutiny, verification, lifecycle,
signing, CI, or delivery evidence because of time, budget, pressure, or failure.

Before creating review state, call `workspace-reader.status` through the same
connected MCP. Require `final_review_protocol.contract_version >= 3`,
`minimum_clean_iterations: 1`, `scoped_coverage: true`,
`explicit_policy_migration: true`, and
`durable_pending_assignment_recovery: true`. A missing or mismatched capability
accepts zero enforced review credit and blocks terminal delivery/readiness.
Recover by updating the plugin and version-matched binaries, restarting the
harness, then repeating this exact connected status check. Advisory manual
observations cannot replace the coordinator. Independently authorized causal
checkpoint commits and pushes retain their own normal gates.

Existing v2 sessions retain their recorded three-round requirements. Missing
`risk_assessment.coverage_policy` is legacy-client compatibility, not a modern
one-round plan. Use the explicit, independently assessed `final_review.migrate_policy`
operation to change an active legacy contract; never hand-edit state or infer
narrower historical inspection, freshness, or lifecycle evidence.

Apply `model-routing` to every review assignment. Ordinary lens review uses the
substantive route; activated architecture, security, human-safety, ambiguity,
or disputed-verification work and the accountable readiness decision use the
strong route defined by that canonical matrix. The coordinator assigns
architecture, security, and human-safety lenses through the resolved strong
model role used for verification while ordinary lenses use the substantive
review role. An unavailable or inherited route follows the canonical bounded
handoff or blocked-result protocol, never an implicit downgrade.

The canonical `model-routing` skill supplies capability and scrutiny requirements for each assignment. Its optional runtime mapping resolves concrete models and supported reasoning settings without pinning a model generation in the plugin. Project-local `[final_review.models.codex]` entries and explicit plan arguments may override the model role for a phase; these choices must still satisfy task eligibility, capability evidence, scrutiny, and actual runtime selection. Record the concrete route separately from the phase model role required by the attestation. `pre_filter` owns the all-dimension broad risk scout; `post_filter` normally uses the deterministic `final_review.filter_findings` path and makes no model call. The caller starts each assignment as a fresh-context subagent, closes it after receiving the result, and submits the required attestation naming the assigned model role plus `fresh_context: true` and `closed_after_result: true`.

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
checkpoint repeats focused verification and lightweight review, commits through
the installed Lefthook pre-commit gate, verifies the exact commit, message and
signature, and performs its authorized push through the installed pre-push gate.
Do not manually duplicate hook-owned checks. Record required CI runs for the new
exact SHA; any completed required failure enters the existing Tiber recovery hold.
A material delta needs an independent assessment and fresh affected-behavior/
dependency review. Retain proven unchanged receipts and every recorded
responsibility and sample floor: an ordinary delta cannot omit an obligation or
lower its sample count. State these requirements when describing remediation.
A whole reset requires independently established whole-scope impact or unisolatable
provenance; v2 retains its recorded full-round contract. Do not
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
delta assessment and affected-coverage renewal against that new identity.

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
- Require each currently pending selected responsibility and assigned verifier. Normalize schema, lifecycle, provenance, and coverage failures as bounded non-clean results inside the authoritative `final_review.advance` transition. Under v3, replace invalid assignments without credit, preserving independently valid peers. A malformed v3 verifier replaces adjudication and retains authenticated source results without counting them as fresh samples.
- For retained v2, explicitly describe **bounded classification inside authoritative `final_review.advance`**: the coordinator bounds and classifies schema-invalid lens evidence, invalid caller attestations, and malformed/provenance-invalid verifier evidence before recording the non-clean transition. It clears both `clean_streak` and durable verified-clean evidence, closes any pending verifier, and advances to a fresh iteration with the complete selected lens set. No corrected retry preserves the old streak or frozen verifier assignment. Three new consecutive complete finding-free iterations remain required. The v3 adjudication-only replacement does not apply.
- In the source-level standalone Tiber review contract, a current result that
  fails scheduler provenance or finding-identity checks emits durable
  `AssignmentResultRejected`, invalidates that full iteration, and permits only
  fresh next-iteration assignments. That contract is not yet bound to the
  installed Tiber task-board binary.
- Require complete applicable independent coverage. Each selected responsibility
  normally needs one genuine fresh-context result. Additional samples require the
  concrete consequential risk, residual uncertainty, scope, and count rationale
  recorded by the independent risk scout.
- Repairs invalidate affected behavior, dependencies, and checks. Preserve proven
  unaffected peer receipts with their original source identities and a current
  applicability proof. A whole reset requires independently recorded whole-scope
  impact or unisolatable shared provenance.
- Independently rejected or properly dispositioned report-only findings remain
  visible without a ritual finding-free rerun. Confirmed correctness/acceptance,
  security, safety, verification, and delivery blockers remain required work;
  caller defense alone never closes an independent obligation.
- Malformed assigned evidence earns zero credit. Execute the exact fresh pending
  assignments returned by the coordinator. A failed v3 verifier replaces adjudication
  over frozen targets while valid source-review peers remain usable.

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
- Assess progress against the approved goal. When required repairs or fresh independent
  reviews remain and no human decision is necessary, call
  `final_review.continue_review` with the returned `state_ref`, a stable
  `operation_id`, and a concrete `rationale`. This records the assessment and
  opens another 75-minute window. It preserves blockers, clean-round credit,
  required lens coverage, and the original start time. Retry the exact same
  request after an interrupted response; changed requests need a new operation ID.
- Stop for a human only when a specific unresolved decision or action requires
  one. Record that dependency explicitly. A locked signing agent blocks the
  signing operation; continue independent authorized work that can still progress.
- `ship`, `split`, and `escalate` remain explicit choices through
  `final_review.advance` with unchanged `current_diff_hash`, empty `lens_results`,
  and one `review_budget_decision`. `split` is available only for unlanded
  reviews and requires two distinct ticket references. Escalation requires a
  concrete escalation reference; elapsed time alone is not a human dependency.
- Reject `ship` until every planned increment and acceptance criterion is
  delivered, every blocking finding is resolved, and the durable review state
  contains complete valid independent coverage and all required consequential-risk samples. For
  direct-to-trunk or PR/MR mode, also require terminally successful CI for the
  exact final-reviewed pushed SHA with no current completed failed job. For
  local-only mode, instead require fresh readiness evidence bound to the exact
  final-reviewed local identity and require no remote CI. Once valid, `ship` is
  terminal and schedules no more reviewers.
- `split` preserves its explicit hold. A legacy `escalate` hold can be resolved
  with `final_review.continue_review` plus `recovery_reference` explaining why
  the recorded dependency is resolved. This resumes the existing obligations;
  it grants no review or verification credit.

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
| No explicit base               | ticket-start baseline to the complete tracked worktree  |
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

Select `production-risk-footguns` when concrete deployment, resource-use, data-access, unsafe-default, or production-scale behavior warrants it. Model/document review and internal tooling do not acquire production obligations from a filename or mention of a future production system. Security and safety remain independently applicable wherever actual trust boundaries or plausible harm are involved.

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
record the finding and honest disposition; preserve independently valid unaffected
coverage under v3 and replace only invalid or affected obligations. V2 retains its
recorded full-round recovery. Separately verify the ticket's actual acceptance criteria; and require `final_review.advance` to
report complete valid independent coverage with no
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
observation must be recorded and dispositioned honestly. V3 can credit genuinely
valid independently adjudicated or report-only coverage without a ritual restart;
caller defense and malformed evidence earn no credit. V2 retains its recorded
full-round finding-free requirement.

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

1. Pin the immutable ticket-start baseline and exact delivered source inventory.
   Collect fresh repository-required verification and impact-selected regression
   evidence. Remote readiness still requires terminal-success CI for the exact
   reviewed SHA and real CodeRabbit evidence wherever required; local-only mode
   uses its exact authorized local snapshot. Queued/running/older-SHA evidence is
   insufficient, and completed required CI failure activates recovery.
2. Run the one assigned fresh independent risk scout. It classifies every supplied
   dimension, artifact behavior and intended deployment, all applicable or uncertain
   lenses, scoped paths and dependencies, and configuration/environment/input/
   freshness identity. Return `coverage_policy` with one sample by default. High
   residual consequential risk may justify 2–10 samples for that responsibility;
   exceptional dimensions require at least two and supported trigger evidence.
3. Plan with its actual assessment and source-bound shared verification evidence.
   Execute exactly the current pending assignments. Apply model routing separately
   to every responsibility. Every reviewer receives the complete inventory for
   scope resolution but reviews its assigned behavior and dependencies. Never turn
   administrative batching into new delivery branches, recursive splits, or tickets.
4. Collect the genuine structured result, close the actual reviewer, then append
   its assigned-model-role, fresh-context, and post-close lifecycle attestation.
   Supply `coverage_evidence.dependency_blobs` for every actually inspected assigned
   path and dependency. The host independently observes raw Git modes and blob
   identities. Missing, forged, stale, reused, or unattested evidence earns no credit.
5. Submit the exact snapshot hash on every advance. Independently adjudicate
   blocking or disputed findings through the returned verifier. Frozen targets,
   assignment provenance, and genuine closure remain mandatory. A rejected finding
   may satisfy coverage; uncertainty or confirmation preserves the obligation.
   Preserve raw allegations, dispositions, defenses, and actual verifier evidence.
6. For a real repair, first complete its causal test/review/checkpoint workflow.
   Submit the delivered changed snapshot, complete path inventory, and fresh bound
   tests with empty lens results. Run the returned independent delta scout. It
   records affected behavior/dependencies, `whole_scope_affected` and concrete
   `invalidation_rationale`, and current source/scope/freshness proofs. Execute only
   the resulting outstanding review assignments. Caller claims cannot preserve
   changed dependencies; the host re-observes them.
7. Visual receipts require the separately declared `rendered-artifact` responsibility.
   A changed source hash prompts impact assessment, not automatic visual discovery.
   To retain a visual judgment across a delta, provide `render_continuity` for the
   complete declared coverage, bound to the current diff and shared-test
   `artifact_reference`: independently checked unchanged render inputs or current
   output equivalence, with unchanged requirements and relevant rendering/freshness
   conditions. Reinspect changed/new/unproven rendered behavior. Semantic contract
   changes still require model/document review even when rendered bytes are identical.
8. Use durable `pending_assignments` after interruption, and retry exact idempotent
   operations after lost responses. A malformed lens result replaces its assignment;
   a malformed v3 verifier replaces only adjudication when peer provenance remains
   valid. Shared provenance compromise invalidates all dependent evidence. The
   coordinator's returned obligations, not generic reset language, govern recovery.
9. Completion requires authoritative coverage of every selected responsibility,
   every justified extra sample, all required verifiers, and no unresolved blocker.
   Run strong readiness analysis against exact-revision evidence. Keep signed
   delivery, real CI/CodeRabbit evidence, and task-board gates where enabled.
   A clean unchanged review creates no extra commit, push, or source mutation.

V2 sessions keep their original full-round policy until an audited migration.
A 75-minute checkpoint remains a progress assessment: supported continuation
preserves obligations, start time, and failure holds. `ship` cannot waive them.

## Output

Before PR creation, merge, or readiness claims, report the scope/baseline,
lenses, fixes/defenses/remaining risk, the selected unrelated-finding
disposition and its out-of-scope report, risk-selected pass evidence, the final
blocking-finding status, and verification commands/outcomes.

## Independent rejection evidence and review yield

Inspect `resolution_history` in each reviewer packet. It retains independently
rejected findings with exact finding/lens identity, rationale, checked evidence,
assumptions, verifier provenance, reviewed scope, and source dependency identities.
Reuse the stable identity for the same failure path; wording similarity does not
establish a duplicate. An unsupported repeat remains a raw non-clean allegation
and a duplicate, without renewed escalation or verifier work.

To challenge a retained resolution, include `resolution_reopen` in the finding
with its exact `resolution_id`, `reason` (`contradictory-evidence`,
`relevant-change`, or `incomplete-prior-verification`), `explanation` of why the
prior resolution no longer holds, and concrete `evidence_ref`. The challenge
requires independent adjudication. Source dependency changes invalidate reuse;
unrelated documentation changes do not. Missing historical dependency evidence
cannot establish reusable rejection credit.

When a carried previously rejected finding has actually changed source
dependencies, submit the preserved complete `lens_results` unchanged. The host
observes the current dependency identities and returns `verifier_required`
for independent reverification of the historical finding. This is not a new
reviewer allegation: do not edit a completed clean result or invent
`resolution_reopen` evidence to make the verifier eligible. Preserve the original
baseline and authoritative `state_ref`; use only the returned assignment.
After the fresh verifier closes, resubmit the same review request with its real
`verifier_result` and lifecycle attestation. A rejection can refresh reusable
resolution evidence only with all actually checked dependency identities; a
confirmed or uncertain result leaves the finding open. Stale assignments,
invented blobs and incomplete results remain invalid. Finding-free submitted
rounds retain normal clean credit after successful adjudication; a newly
submitted allegation still makes its round non-clean even if later rejected.
V3 completion uses valid scoped coverage and justified samples. A retained v2 session still requires its recorded three complete clean rounds until explicit migration.

Independent verifiers rejecting a finding should name the concrete evidence in
`causality_evidence`, state the rationale and actually checked `assumptions`,
and supply all actual repository-relative `dependency_blobs` as Git
`mode:blob-OID` identities. The host re-observes them; invented identities fail.
Do not omit a dependency merely to make a resolution survive a relevant change.
The caller adds the real assigned model and fresh/closed lifecycle attestations
only after the independent verifier has completed.

Use `final_review.yield_report` with the current `state_ref` to report actual
completed lens rounds separately from coordinator iteration, delta and reset
bookkeeping. Report raw allegations separately from confirmed defects,
duplicates, independent rejections, reopenings, and verified repairs. Each
round binds its counts to the reviewed scope, shows source changes, and returns
evidence references. Inspect a reference with `final_review.evidence`; do not
infer counts from iteration numbers or filtered bucket totals. Legacy evidence
that was never retained is unavailable, not zero. An all-clean round supplies
required scrutiny; its yield never changes the review policy automatically.
Report `review_counts` (submitted, accepted, malformed) and `round_attempts`
separately from eligible completed rounds. Inspect the round's `review_attempts`
for each reviewer's submitted status, native disposition, assigned key, scope
binding and caller model/lifecycle attestations, including clean reviewers and
accepted peers of a malformed report. Concrete `actual_model` is unavailable;
never substitute the attested `model_role`. When full review counts are null,
label `observed_review_counts` as retained evidence only and state the missing
legacy/pruned coverage.
A malformed findings container leaves raw allegation totals null while bounded
reviewer attempts remain inspectable; label `retained_raw_allegations` as a
partial count. An absent submitted status is unavailable, even when the native
disposition is malformed.
