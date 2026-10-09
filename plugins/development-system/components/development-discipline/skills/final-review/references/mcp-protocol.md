# Final Review MCP Protocol

Read this reference only when running, debugging, or changing the MCP-enforced
final-review path.

## Policy version and scoped coverage

Read [proportional review policy](proportional-review-policy.md). V3 default
completion requires one complete risk-selected round and only independently
justified extra samples for consequential responsibilities. `risk_assessment`
contains `coverage_policy` with artifact classification, freshness identity, and
scoped requirements. Every v3 lens result contains source-observed
`coverage_evidence`; assigned results use `final-review-lens-result-v2`.

V3 delta assessments contain `coverage_policy`, `whole_scope_affected`, and
`invalidation_rationale`; their schema is `final-review-delta-risk-assessment-v2`.
Optional `render_continuity` proves unchanged render inputs or current output
equivalence for separately scoped visual coverage against the current diff and
shared-test artifact reference. The host invalidates changed dependencies and
retains original peer receipts only with current applicability proof.

V2 records and clients without `coverage_policy` retain their recorded three
complete clean rounds. Changing that contract requires the independently
assessed, idempotent `final_review.migrate_policy` operation. Its first call
returns a fresh risk assignment without applying migration; after actual scout
closure and truthful `prior_assignment_closures`, resubmit with the assessment.
Keep baseline, history, blockers, and holds. Missing historical dependency or
freshness bindings cannot become narrower reusable v3 credit.

## Required Scope

Before `final_review.assess_risk`, call `workspace-reader.status` through the
same connected MCP and validate its `final_review_protocol` attestation. The
current contract requires `contract_version >= 3`,
`minimum_clean_iterations: 1`, `scoped_coverage: true`, `explicit_policy_migration: true`, and
`durable_pending_assignment_recovery: true`. Missing or weaker fields identify
a stale skill/runtime pairing: create no review state, accept zero clean
iterations, and reject delivery. Install the current-host binaries from the
updated marketplace checkout with `just install-development-system-binaries`
(or `scripts/install-development-system-binaries.sh`), update the Development System plugin, restart the harness, and
start a new session only after the same-MCP attestation passes. A valid
plan must also return coordinator-owned state whose
`required_clean_iterations` is at least the attested minimum; otherwise discard
that session as a protocol mismatch.

Run `final_review.assess_risk` before `final_review.plan`; the plan boundary
requires that scout's bound `risk_assessment`, the complete reviewed
`changed_files` inventory, a non-placeholder `diff_hash`, the full
`baseline_commit`, and the same diff-bound `shared_test_evidence`. A caller
cannot fall back to an all-lens legacy plan by omitting the scout. Before the
ticket's first commit or push, resolve and retain the full baseline OID.
Incremental pushes can move the named base past the ticket, so final review must
not re-resolve that ref. Supply the same full OID to the scope-hash helper,
`final_review.assess_risk`, and the risk-planned `final_review.plan`; missing,
symbolic, abbreviated, or changed baseline values fail closed. Compute both
`diff_hash` and every
`current_diff_hash` with the plugin's
`scripts/final-review-scope-hash.sh` helper. Write the complete current
changed-file inventory to a temporary NUL-delimited file and pass its path with
`--changed-files-from` and the retained OID with `--baseline-commit`; do not
expand paths into helper argv. The helper bounds,
validates, normalizes, and deterministically chunks that inventory so its own
Git subprocesses remain below platform argument limits. It hashes the exact
baseline, base-to-index diff, index-to-worktree diff, and current mode/content
manifest for exactly those paths. This makes staged, unstaged, deletion, and
untracked-content changes observable while excluding unrelated local dirt.
Discover tracked paths with the same one-revision content scope using
`git diff --name-only -z --find-renames --find-copies --end-of-options
<baseline_commit> --` for both base and uncommitted scope. Parse those
NUL-delimited records as exact
paths rather than extracting names from the human-readable diff. Discover
untracked and other worktree paths with
`git status --short -z --untracked-files=all`, parse its NUL-delimited records as
porcelain-v1 records, then exact-byte-deduplicate both sources. A primary status
record is `XY ` followed by the raw path, so remove exactly the first three
bytes. If either status byte is `R` or `C`, consume the next NUL field as the
source path; `-z` emits destination then source without an arrow. Keep the
actual destination and source pathname bytes, not status prefixes, separators,
or quoted display forms. A clean status does not erase committed base-scope
paths.
Delete the inventory file after each call. Treat helper failure as a blocked
review; do not replace it with an ad hoc Git-diff hash.
Treat the complete `final_review.assess_risk` argument object as an immutable
replay contract. To call `final_review.plan`, reuse every scope, request,
acceptance-criteria, concern, model-role, and evidence field unchanged, then
add the scout's `risk_assessment` result and its caller attestation. The scout
assignment alone intentionally does not duplicate all caller-owned review
context. If the scope changes, recompute the scope hash and start a fresh risk
assessment rather than retrying a stale handoff.

## Completed review and delivery

The pinned Git baseline plus the exact in-repository changed-file inventory are
the only inputs to reviewed source scope and its hash. Coordinator events,
snapshots, state files, and projections are bookkeeping outside that inventory.
A coordinator write, snapshot update, content-identical commit, or
staging-partition change cannot by itself start another source-content review.

The scope hash is deliberately stage-aware during an active review. Moving
unchanged content between the worktree, index, and `HEAD` can therefore change
the hash, and callers must continue sending the fresh helper output on every
`final_review.advance` call. Do not weaken or replace that contract.

Terminal review starts only after every planned implementation increment and
every actual acceptance criterion have been completed and checkpoint-delivered.
It consumes that ticket-complete identity: the exact pushed SHA for remote modes
or the exact local snapshot for local-only. Retain its pinned ticket-start
baseline, requested scope, exact path inventory, and reviewed path
content/modes. A clean unchanged review adds review evidence only; it creates no
commit, empty commit, push, or replacement local checkpoint.

Any changed reviewed path, content, mode, untracked content, pinned baseline,
or requested scope is source-changing remediation. Complete its mode-specific
immediate-test, lightweight-review, and commit-through-Lefthook checkpoint first, including exact
commit, message, and signature verification for remote commits, then perform a
independent delta assessment and fresh review of affected behavior and dependencies.
Preserve proven peers under v3; full renewal requires material whole-scope impact
or unisolatable provenance. V2 retains its recorded full-reset contract. In remote modes, comprehensive suites remain in CI; review
may proceed while exact-SHA CI is pending, but a completed failure preempts
through recovery and readiness requires terminal success for the exact
final-reviewed pushed SHA. Local-only instead requires fresh readiness evidence
bound to the exact final-reviewed local identity, requires no remote CI, and
never invents a remote action. A local-only remediation commit is permitted only
when authorized. If repository policy requires committed evidence but the user
explicitly withholds commit authority, block without committing or pushing.

`final_review.advance` also validates scope state; when `current_diff_hash`
differs from the stored hash, provide `current_changed_files` so the next review
iteration sees the current diff.
Session identifiers are bounded to 128 characters, and requested clean
iterations are bounded to 3-10 to cap assignment fanout.
The coordinator returns both the legacy full `state` and a compact `state_ref`.
Prefer `state_ref` between calls; it contains only session identity, storage
scope, and the authoritative-state fingerprint. Never summarize or reconstruct
the legacy state. The coordinator stores full project-scoped state as EventCore
facts. The advisory plugin uses a separate local-only Git authority under the
repository's Git metadata and never publishes those review facts to a remote;
standalone Tiber's native workflow service uses its configured Development
Workflow authority. SQLite in user state is a rebuildable report/projection,
not the decision authority. A new stdio MCP process resolves a
valid reference, including pending verifier and delta-risk assignments. If a
caller loses the latest handoff, `final_review.resume_latest` returns the latest
reference plus the compact pending-assignment summary from `session_id`,
`project_root`, and `work_item_id` when the original review was ticket-bound,
without advancing the review. Keep those binding fields together; a reference
whose session, project, or work item came from different handoffs is rejected.

Call `final_review.pending_assignments` with `state_ref` when a plan or advance
response was truncated or too large to retain. Its versioned compact summary
returns every pending assignment's exact `subagent_key`, exact `model_role`,
phase metadata, close policy, result-schema version, and prompt reference. Lens
summaries also carry the lens, iteration, and shared-test-evidence ID; verifier
and delta-risk summaries carry their durable assignment identity. It never
returns full prompts in the summary. Pass one exact `subagent_key` with the same
`state_ref` to retrieve only that assignment's original durable prompt and
result schema. Repeated summary or prompt retrieval is idempotent: it appends no
event, changes no revision, fingerprint, scope, or hash, and never reassigns a
lens or model role. Large delta artifacts are immutable, project-scoped cache
files bound to their recorded snapshot commits and blob digest. Versioned patches
use canonical repository-relative headings; `changed_paths` stays relative to the
bound `project_root`, including a native project rooted in a Git subdirectory.
Retrieval can
reuse intact digest-matching files without regenerating them after Git
presentation settings change. New evidence records deterministic snapshot-bound
rendering; missing files can be rebuilt with that renderer. Historical ordinary
and replay temporary locators may be relocated while assignment identity stays
unchanged. Missing unversioned evidence must reproduce its exact recorded digest;
if historical custom rendering is unavailable, follow the actionable recovery
hold instead of substituting newly rendered evidence.
Inspect the returned artifact and digest before dispatch; never reconstruct it
from the current working tree or count retrieval as a review round.

If source genuinely changes again while a delta scout is pending, submit
`final_review.advance` with the current `state_ref`, the true new
`current_diff_hash`, complete `current_changed_files`, and fresh
`current_shared_test_evidence` with a new evidence ID bound to that hash. Supply
empty `lens_results`, and no caller decisions, delta assessment, verifier result,
or budget decision. The coordinator verifies a changed captured snapshot
against the same prior baseline, preserves old evidence/history, closes the
superseded scout, and issues a fresh independently reviewed delta assignment.
A changed label with unchanged captured source is rejected. This transition
adds no clean credit and does not replace any required lens review. Never pass
an old target hash as the current hash to finish a stale assignment.

A pre-upgrade pending verifier or delta-risk record that
lacks the durable assignment fails explicitly with `recovery=restart_final_review`
instead of returning an empty or reconstructed assignment.

Missing or invalid lifecycle/model attestation is malformed assigned evidence,
not passing credit. In v3, reject that assignment and execute the exact returned
fresh pending scope; preserve independently valid peers. A malformed v3 verifier
closes its assigned context and requests a fresh verifier over frozen targets,
without repeating source discovery or counting retained results as new reviewers.
Shared provenance compromise invalidates every dependent receipt. For retained
v2, `final_review.advance` bounds and classifies invalid lens or verifier evidence,
records a non-clean iteration, clears the streak and durable verified-clean
credit, closes the pending verifier, advances the iteration, and assigns the
complete selected lens set afresh. Three new consecutive complete finding-free
iterations remain required; correcting the old result cannot preserve credit.

The equivalent standalone-Tiber transition emits durable
`AssignmentResultRejected` when a current result fails scheduler provenance or
finding-identity checks. That fact invalidates the full iteration, clears the
consecutive-clean streak, and permits only fresh next-iteration assignments.

Creation is insert-only and every
transition uses a durable revision compare-and-swap, so concurrent processes
cannot admit duplicate sessions or overwrite each other's progress. Unknown,
evicted, stale, or mutated state fails closed with sanitized expected/received
fingerprints and a restart, resume, or abandon recovery action; advancing a
completed session also fails. Each process retains at most 32 active sessions,
and durable storage is bounded independently.

`final_review.plan` rejects a risk assessment that selects no deep-review lens.
A v3 review reaches completion only with sufficient valid receipts for every selected responsibility, every justified extra sample, all required verifiers, and no unresolved blocker. A retained v2 review requires its recorded complete clean rounds. The
plugin's advisory surface returns no native workflow handoff; stop without
calling one. Standalone Tiber may name `next_tool:
workflow.record_clean_review`; only when that native workflow service is
actually available, pass the terminal plan's `state_ref` as
`review_state_ref`.
When an advance returns `verifier_required`, the server retains the pending
assignment ID and exact core pre-verifier arguments. Until the caller resubmits
the same lens, scope, and caller-decision arguments plus the matching
`verifier_result`, missing verification or changed core arguments fail closed.
The resubmission may also add `unrelated_follow_ups` or
`security_escalations` newly required by the verifier's final classification;
those disposition records are deliberately outside the frozen core. Pending
verifier state is cleared only by an accepted advanced transition or session
eviction.
Call `final_review.filter_findings` before the initial advance, prepare any
applicable `caller_decisions` from its retained findings, and include them on
that first advance. Once `verifier_required` is returned, introducing a defense
or accepted-risk decision changes the frozen core arguments and fails closed;
only the verifier result and conditionally required disposition documentation
may be new.
Reviewer prompts include a bounded changed-file navigation hint plus an
authoritative `scope_reference` containing project root, scope, base, and diff
hash. Its argv-based scope resolution uses a one-revision base-to-worktree diff
plus worktree status for untracked discovery; triple-dot, index-only, and bare
worktree diffs are incomplete substitutes. Reviewers must inspect that complete
repository change set instead of treating a truncated inline file hint as the
full scope. The project root and other scope fields are bound into
`review_contract_id`; planned states without a valid contract cannot advance.
Conditional lenses use objects with both an identifier and a reviewer objective:

```json
{
  "conditional_lenses": [
    {
      "id": "agent-instruction-quality",
      "description": "Check whether agent instructions are complete, focused, and executable."
    }
  ]
}
```

The MCP includes a concrete objective in every default and conditional lens
assignment. Identifier-only conditional lenses fail closed.

## Imported Defenses

When accepted defenses predate the current MCP process, import them on
`final_review.plan` instead of relying on caller conversation context:

```json
{
  "prior_defenses": [
    {
      "id": "bounded-cache",
      "lens": "production-risk-footguns",
      "decision": "defended",
      "defense": "The cache is request-scoped and has a fixed entry limit."
    }
  ]
}
```

The lens must be part of the planned default or conditional lens set. IDs,
rationales, total entries, and entries per lens are bounded; malformed,
duplicate, unknown-lens, or rationales without a non-whitespace character fail
closed. Imported defenses are bound into the initial review contract and
included in the first assignment for their matching lens.

## Exceptional-Risk Evidence

The scout's `exceptional_triggers` array accepts only these exact values:

- `destructive-or-irreversible-operation`
- `authentication-or-authorization-boundary`
- `sensitive-data-migration`
- `cryptographic-behavior`
- `safety-critical-behavior`

An `exceptional` overall profile requires at least one supported trigger and at
least one dimension explicitly assessed as `exceptional`. Only those explicitly
exceptional dimensions receive two independent passes; every other selected
dimension receives one. A supported trigger may appear on a lower overall
profile when the scout's concrete evidence shows that mitigations keep the risk
below exceptional. Unknown, non-string, duplicate, missing, or empty trigger
evidence fails closed when applicable. The same validation applies to delta
assessments, which preserve the union of prior and replacement-diff trigger
evidence in authoritative state.

Risk-scout findings pass through the same deterministic relevance classifier as
lens findings before they can affect persistence, disposition, blockers,
verification, follow-up tickets, or discovery resets. Request,
acceptance-criteria, and explicit-concern claims require exact `matched_context`;
cross-cutting claims require `changed_diff_evidence` bound to an in-scope changed
path; prior-defense challenges require the accepted defense ID plus new
contradictory changed-diff evidence. Missing, mismatched, generic, or out-of-scope
claims remain report-only in authoritative state. Initial and delta scouts use
the same contract.

## Scope-Growth And Review-Budget Gates

The broad risk scout evaluates whether the current ticket has grown into either
of these scope triggers:

- `new-subsystem`: added scope outside the original request or acceptance
  criteria crosses a runtime, ownership, or delivery boundary and has its own
  acceptance contract plus independent build, test, and shipping mechanisms;
- `unusually-broad-diff`: the work contains at least two internally cohesive,
  low-coupling increments that can be accepted and shipped independently.
  File/path count, generated churn, and diff size alone are not evidence.

A candidate is cohesive when its paths serve one acceptance contract.
Candidates are low-coupling when either can build, test, and ship without the
other's unfinished behavior. If valid candidates cannot be constructed, the
scout uses review batching and must not assert a scope-growth trigger merely
because the review surface is large.

The initial risk assessment and plan declare `review_lifecycle` as `landed` or
`unlanded`; the coordinator propagates it through delta reassessment. A child
split also carries `split_lineage`, binding its root and parent work item IDs,
generation, and source diff hash. Generation one is the maximum: the
coordinator rejects every further split from a generation-one child, even if
its diff changes, instead of returning more candidates.

For unlanded work, either trigger requires `split_required: true`, a nonblank
`split_rationale`, and 2-16 `split_candidates`. Candidate IDs are unique bounded
identifiers. Each candidate includes a title, normalized `scope_paths`,
acceptance criteria, `independently_shippable_reason`, and structured
`delivery_boundaries` for independent build, test, and shipping evidence. Fully
overlapping ownership, path aliases, bare paths, and synthetic path-filtered
scopes are insufficient. Combined ownership must still cover `changed_files`.

The coordinator validates this structure and persists a contract-bound
`scope_split_hold`. Initial planning and delta reassessment return
`split_confirmation_required`, no assignments, and an authoritative preview
with tracker mutation and blocking dependencies disabled. The caller must show
the preview and obtain explicit user confirmation before calling
`final_review.confirm_split`. `delivery-tickets` authorizes tickets but forbids
blocking dependencies. `delivery-tickets-with-blocking-dependencies` also
requires a bounded causal prerequisite reason. Administrative review ordering
is never sufficient.

When `review_lifecycle` is `landed`, scope growth produces retrospective review
batching, not delivery decomposition. The coordinator retains review work and
does not authorize tracker mutation. Callers must not create a review-only
branch, synthetic path branch, recursive tickets, or blocking dependencies for
administrative review. Concrete unresolved defects may still become ordinary
follow-up tickets. Because the server retains held sessions, callers cannot
weaken or replay their way around these rules. The scope-split decision takes
precedence over a simultaneously due review-budget checkpoint.

Every risk-planned session carries a contract-bound, server-timed review budget.
For a medium-risk session, the checkpoint is exactly 75 minutes after planning
(inside the policy's 60-90 minute range). It also activates if a delta scout
raises a lower-risk session to medium; the original planning time remains the
clock origin. Sessions that start high or exceptional use their own bounded
review plan rather than this medium-risk checkpoint. Elapsed time saturates at zero if the wall clock
moves backward, and callers cannot supply or mutate the start time.

When a lens or delta transition reaches the checkpoint, the coordinator first
applies its findings and replacement-diff evidence, then returns authoritative
state with `advance_kind: review_budget_checkpoint`,
`checkpoint_pending: true`, no next assignments, and the allowed decisions.
This ordering prevents a later decision from dropping already-submitted
findings.
For autonomous continuation, call `final_review.continue_review` with the exact
returned `state_ref`, a stable `operation_id` and a nonblank progress `rationale`.
The operation preserves the original start time and obligations, records an
assessment in durable history, and sets the next checkpoint 75 minutes after
that assessment. The exact request is idempotent after restart or lost response;
reuse with changed arguments fails with an operation conflict. A new operation
requires the current reference and a pending timed checkpoint. A legacy
escalation hold additionally requires `recovery_reference` explaining resolution
of the recorded dependency. Split holds and completed sessions require their
specific supported transitions instead.

A timed checkpoint requires a progress assessment, not automatic human
permission. Continue required repairs and fresh reviews when authorized work
can progress. Ask a human only about a concrete unresolved dependency; signing
unavailability blocks signing, while independent work continues.
The next `final_review.advance` call must keep the current diff hash,
send empty `lens_results`, and add one `review_budget_decision`:

```json
{
  "decision": "ship",
  "rationale": "Acceptance criteria and review gates are satisfied."
}
```

`decision` is exactly `ship`, `split`, or `escalate`. `split` additionally
requires 2-16 distinct nonblank `ticket_references`; `escalate` requires a
nonblank `escalation_reference`. The coordinator rejects premature, duplicate,
malformed, or diff-changing decision calls. `ship` is rejected while any
planned increment or acceptance criterion remains undelivered, any known
blocking finding remains, or required independent coverage and justified samples remain incomplete; legacy v2 also requires its recorded complete finding-free
iterations exist. In direct-to-trunk or PR/MR mode it is also rejected until CI
succeeds terminally for the exact final-reviewed pushed SHA with no current
completed failed job. A successful build for an older revision never satisfies
that gate; a queued or running build for the reviewed SHA remains pending. Any
current remote build with a completed failed job activates
`ci-failure-follow-up`, which takes precedence and requires exact diagnosis plus
terminal success before release or new work. In local-only mode, `ship` instead
requires fresh readiness evidence bound to the exact final-reviewed local
identity and no remote CI. A valid `ship` decision is terminal for final review,
returns `complete: true`, and schedules no reviewers; it cannot discard
remaining lens work or substitute for any mode-specific gate.
`split` and `escalate` persist contract-bound holds, preserve every blocker and
reject ordinary advances while held. The supported continuation operation can
resolve an escalation hold with explicit recovery evidence; it cannot resolve a
split hold or waive clean rounds, independent attestations, security or delivery
gates.

## Finding Disposition And Escalation

Risk-planned review blocks only caused or worsened `CRITICAL`/`MAJOR` findings
whose evidence names the in-model actor/input or initiating failure, trust or
hazard boundary, affected asset/person, causal mechanism, intended deployment,
and `major` or `critical` security/safety impact, plus an in-scope changed
remediation path. Incidental or
pre-existing findings at those severities, caused non-security/non-safety
findings, and every `MINOR` finding require a matching
`unrelated_follow_ups` backlog reference before advance. `TRIVIAL` findings are
report-only. Acceptance criteria remain an independent completion gate.

Review severity is exactly one of `CRITICAL`, `MAJOR`, `MINOR`, or `TRIVIAL`
and is separate from both `security_impact` and `safety_impact` (`none`,
`minor`, `moderate`, `major`, `critical`). Prioritize deferred work against the
whole backlog using value, risk, likelihood, and opportunity cost. Already
tracked findings on an unchanged diff are neither re-verified nor re-ticketed
unless new evidence materially raises severity. Any suspected PII exposure or
pre-existing/incidental major/critical security or safety observation requires
an appropriately high-priority documented ticket; never silently discard it.

## Model Routing

The MCP resolves model labels/roles for:

- `pre_filter`
- `lens_review`
- `post_filter`
- `verifier`

Resolution precedence is:

1. explicit `final_review.plan` tool arguments, including either
   phase-specific arguments or `model_roles.{phase}`;
2. project-local `.development-discipline/final-review.toml`;
3. Codex defaults;
4. generic abstract roles.

A present explicit override must be a nonblank string using the allowed model
label characters. Non-string, blank, or unsafe scalar and
`model_roles.{phase}` values fail closed instead of falling through to a lower
precedence source.

Project TOML shape:

```toml
[final_review.models]
pre_filter = "strong-reviewer"
lens_review = "substantive-worker"
post_filter = "bounded-helper"
verifier = "strong-reviewer"

# Optional concrete Codex overrides belong in project-local configuration.
# Resolve each assignment’s concrete route using model-routing.

```

Top-level phase values are abstract roles. The optional `codex` table overrides
them one phase at a time with concrete Codex model IDs.

Legacy non-risk-planned sessions can add
`[final_review.dispositions.<SEVERITY>]` tables for `CRITICAL`, `MAJOR`,
`MINOR`, and `TRIVIAL`. Each table must map every configured review lens to
exactly one of `block`, `ticket`, `document`, or `ignore`; incomplete, unknown,
or invalid entries fail closed. Risk-planned sessions retain that configuration
for contract compatibility but use the mandatory deterministic disposition
rule: only caused/worsened material security or human-safety findings block,
other nontrivial findings require backlog evidence, and TRIVIAL is report-only.

Resolved roles, their sources, required clean count, lens objectives, and the
caller-attestation policy are bound into `review_contract_id`. Mutating them or
any progression field in caller-carried state makes the server-authoritative
session check fail closed.

When project TOML supplies any model role, the MCP marks model-role
confirmation as required. The required post-shutdown `caller_attestation` is
that confirmation: it records the actual assigned role, fresh context, and
subagent closure. Treat configured labels as caller-side routing configuration,
not permission for the MCP server to spawn models or subagents.
If project TOML exists but is malformed, unreadable, or outside the project root
after symlink resolution, the MCP fails closed instead of silently falling back.
Model labels are bounded identifiers rather than free-form prompt text; control
characters, whitespace, and prompt delimiters are rejected for both tool
arguments and TOML values.

## Phase Execution

Model roles do not imply one model call per phase:

The existing `verifier` configuration key is intentionally the canonical
strong-review role for this workflow, not a verification-only role. It governs
both conditional batched verification and the architecture, security, and
human-safety lens assignments that require the strong route. Projects that
override `verifier` therefore change all of those strong responsibilities
together; the current schema does not expose an independently configurable
strong-lens model. This keeps one source of truth for the harness-resolved
strong route and avoids
an apparently independent setting that could silently drift.

- `pre_filter` owns the mandatory all-dimension broad risk scout and any
  optional assistance for a large or noisy scope. Because the scout assesses
  security and human-safety risk, this role uses the strong-responsibility
  route. The scout selects lenses from concrete risk. Optional assistance may
  focus context but must never omit a lens selected by that bound risk plan.
- `lens_review` is one MCP-assigned caller subagent for every ordinary lens and
  iteration. Architecture, security, and human-safety lenses use the canonical
  strong `verifier` role described above.
- `post_filter` is the deterministic `final_review.filter_findings` path by
  default, so its model label is normally not invoked.
- `verifier` is one conditional batched caller subagent when post-filtering
  leaves actionable or needs-human-decision candidates, or when a new
  `MAJOR`/`CRITICAL` security or human-safety finding has material impact but
  uncertain causality. A missing verifier result blocks the transition. A
  failed verifier retains every candidate, and an uncertain result keeps every
  blocking or materially uncertain security or human-safety candidate open.

When `final_review.advance` returns `transition_status: verifier_required`, run
the returned assignment with its exact `subagent_key` and `model_role`, close
the subagent after collecting its result, then resubmit the same lens results
with `verifier_result`. Verified results require exactly one `confirmed`,
`rejected`, or `uncertain` verdict per candidate, a final review severity, and
a non-empty rationale. The server records reviewer and verifier severities and
routes with the verifier's final severity and causality/impact classification.
Rejected candidates do not become unresolved blockers. Under v3, genuine independent
rejection can credit its valid source review without rediscovery; frozen verifier
retries never count as new source samples. V2 finding-bearing rounds remain
non-clean and retain their three later complete finding-free round requirement.
Uncertain blocking candidates and materially uncertain security or human-safety
candidates stay open for human decision, while a verified nonblocking downgrade
requires the applicable backlog/report disposition.

The coordinator accepts at most 23 lens results, 64 findings per lens, 256
findings per iteration, and 256 verifier verdicts. Runtime checks mirror the
public schemas and run before classification or coverage matching. Finding and
verdict matching use indexed lookups so accepted maximum-size batches remain
linear rather than quadratic.
Retained state is bounded to the latest 64 finding-history records, 64 caller
decisions, and 8 defenses per lens. This preserves useful review continuity
without letting a noisy cycle grow until the absolute state-size limit becomes
its normal failure mode.

The MCP enforces complete lens result/key sets, matching verifier assignment
metadata, verdict coverage, transition state, and the clean-iteration rule. It
cannot prove actual model invocation, fresh context, or process shutdown while
remaining prohibited from spawning agents; the calling agent records those
runtime facts after closing each subagent:

```json
{
  "caller_attestation": {
    "model_role": "substantive-worker",
    "fresh_context": true,
    "closed_after_result": true
  }
}
```

Append this object to every lens result before `final_review.advance`, and to a
verifier result after closing the verifier. Missing or mismatched attestations
produce the authoritative non-clean reset transition described above.

## Protocol Versions

Initialization negotiates the requested MCP version. The server supports
`2024-11-05`, `2025-03-26`, `2025-06-18`, and `2025-11-25`; unsupported versions
receive a structured invalid-params response listing supported versions.

## Packaging

Marketplace installs require the current-host Development System executable.
From the marketplace checkout, run `just install-development-system-binaries`
(or `scripts/install-development-system-binaries.sh`). Linux x86_64 downloads
the exact plugin-version GitHub Release and verifies its SHA-256 sidecar and
fixed archive layout; hosts without a prebuilt release build both locked Cargo
workspaces. The installed `SessionStart` hook verifies the installation marker
against the plugin version and automatically repairs missing or stale binaries
before normal MCP use. Direct MCP launchers resolve only those installed
executables and never compile as a side effect of an individual invocation.
Incoming stdio requests and
conditional-lens fanout are bounded so malformed or bursty callers cannot grow
coordinator memory or review-agent count without limit. On any oversized stdio
frame, the server stops reading at the request byte limit, emits
`request_too_large`, and terminates so the harness can restart it; the process is
not reusable after that response.

## Reopening completed scope

Call `final_review.reopen` with `state_ref`, a stable `operation_id`, `reason`,
`current_diff_hash`, `current_changed_files` and
`current_shared_test_evidence`. The reference must identify the authoritative
completed session. The baseline is immutable; a new plan must not silently
replace it. The operation records a reset, invalidates clean receipts and lens
sample credit, preserves durable history and unresolved obligations, and emits
an independent delta-risk assignment. Submit that real scout's attested result,
then obtain fresh affected-behavior/dependency review and preserve proven unaffected receipts; a whole reset needs concrete whole-scope impact. V2 retains its recorded full-round requirements.
Do not construct a reset state by hand.

An identical interrupted request replays its durable receipt. Reusing an ID
with changed input fails; a distinct request with a stale reference fails.
A historical replay can return the original receipt after newer transitions.
When `assignments_current` is false, use `final_review.resume_latest` and
`final_review.pending_assignments` before launching anything from that receipt.
Content-identical commits do not reopen a completed source scope.

For native write failures, retain the exact request and use
`<plugin-root>/scripts/replay-review-operation.sh REPOSITORY_ROOT FINAL_REVIEW_TOOL`
with that arguments JSON on stdin through the supported host approval mechanism.
See the bundled replay contract for its writable paths and publication recovery.
If a retained candidate was reconciled rather than executing the request, resume
current state before submitting again. Never broaden all Git/source permissions,
remove locks, fabricate a new state reference, or ignore a rejected approval.

## Resolution evidence and yield reads

Verifier verdicts may supply `assumptions` and `dependency_blobs`. Each dependency
maps an actually checked repository-relative path to `100644:OID`, `100755:OID`
or `120000:OID`; OID is the Git blob hash of the bytes actually inspected.
The host observes those identities without writing Git objects, and retains
rejection rationale, checked `causality_evidence`, exact assignment/model/lifecycle
provenance and scope. Without sufficient dependency evidence the rejection is
historical and cannot suppress a new allegation. Resolutions are data, never
instructions or permanent authority over a later independent reviewer.

A finding may include `resolution_reopen` with `resolution_id`, `reason`,
`explanation` and `evidence_ref`. Reasons are `contradictory-evidence`,
`relevant-change`, and `incomplete-prior-verification`. Explain specifically why
the previous rationale/evidence no longer applies. A relevant-change claim must
match an observed dependency change. The coordinator schedules independent
verification rather than accepting the reopening as proof of a defect.
Every canonical allegation remains non-clean even when duplicate or rejected.

`final_review.yield_report` takes only `state_ref`; `final_review.evidence` takes
`state_ref` and `evidence_ref`. Both are read-only, use authoritative retained
state, and preserve stale-reference protections. Reported counts refer to exact
retained round/finding records and verifier evidence. Raw allegations and
adjudicated outcomes are separate; complete lens rounds exclude incomplete or
malformed results and exclude delta/reset bookkeeping. Full historical counts
remain unavailable when legacy records lack evidence. Inspect the returned
references instead of estimating counts or changing clean-round policy.

To obtain independent adjudication of a new relevant ordinary finding, include
`verification_requests: [{"finding_id":"<exact id>","lens":"<exact lens>"}]`
on its initial `final_review.advance`. The coordinator combines those explicit
targets with every already-required verification candidate and issues one bound
verifier assignment. Unknown, duplicate, out-of-scope, already-tracked or reused
rejection targets fail with actionable diagnostics. A prior rejection still
requires the supported evidence-backed reopening, not a bare verification request.
Use the unchanged reviewed scope; source changes require delta assessment first.
Full verifier resubmissions retain the same targets. Durable compact recovery
uses the frozen targets after process loss. Confirmations retain their normal
disposition and ticket requirements; independent rejections retain evidence for
subsequent packets. V3 adjudicated coverage does not require ritual source rediscovery. Legacy v2 finding-bearing rounds remain non-clean and require three
later complete finding-free rounds.

Version-two round evidence derives `source_changed` from immutable captured Git
tree identities and retains both snapshot commits and tree IDs in
`source_change_evidence`. A changed caller scope hash or staging partition alone
does not prove source changed. Missing snapshot evidence and historical
version-one hash-derived claims report `source_changed: null`; inspect the raw
historical record without treating its old boolean as verified source evidence.

New round records also retain an optional compact `review_attempts` ledger.
`review_counts` separates submitted, native-accepted and malformed lens reports;
`round_attempts` counts captured review batches. These differ from eligible
`completed_lens_rounds`, clean credit and coordinator iterations. A malformed
lens leaves valid sibling attempts inspectable. The round evidence reference
includes each attempt's submitted status, assigned and submitted subagent keys,
native scope binding and disposition/reasons, and the caller's exact
`model_role`, `fresh_context` and `closed_after_result` claims, including clean
reviewers. `actual_model` is null because the protocol does not independently
record the concrete runtime model; never infer it from `model_role`.
`observed_review_counts` and `observed_round_attempts` cover only retained
ledger-bearing rows. Full review counts are null if any legacy row lacks a
ledger or earlier rows were pruned; `review_counts_available_for_full_history`
reports that separate coverage. No history is retrofilled from iteration
counters or old artifacts. Empty delta/reset bookkeeping adds no attempt.

A bounded report with an invalid findings container still retains its attempt
and valid sibling attempts. Its round records `raw_findings_complete: false`;
raw allegation counts are null because the complete number is unknown.
`retained_raw_allegations` labels only the individually retained allegations,
not the missing total. An empty retained array then cannot establish
`finding_free` or clean credit. Missing submitted status is null, distinct from
the native malformed disposition. Oversized or non-array whole batches remain
unavailable when individual submitted reports cannot be recovered.

The existing serialized-state size limit remains in force. Evidence-heavy
sessions fail closed when that limit is exceeded; no evidence is silently
invented or dropped to authorize completion.

New delta artifacts use `git-snapshot-v2`: the captured project subtree produces
both patch and inventory without expanding the declared file set into Git argv.
Version-one and unversioned legacy rendering remain available for exact recorded
digest recovery. Verified existing artifacts are reused before reconstruction.
Patch generation defaults to a 64 MiB limit; each project retains at most 256 MiB
of published patch artifacts. Set positive byte limits with
`DEVELOPMENT_SYSTEM_DELTA_ARTIFACT_MAX_BYTES` and
`DEVELOPMENT_SYSTEM_DELTA_CACHE_MAX_BYTES` when the required scope needs more.
Budget exhaustion is an actionable hold, grants no review credit, cleans partial
candidates and preserves existing evidence. No referenced history is silently
deleted. Durable Git snapshot and event history remains separately stored;
archive verified cached patches only while retaining the recorded snapshots
needed for deterministic reconstruction.
