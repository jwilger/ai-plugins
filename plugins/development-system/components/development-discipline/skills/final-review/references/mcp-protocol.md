# Final Review MCP Protocol

Read this reference only when running, debugging, or changing the MCP-enforced
final-review path.

## Required Scope

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
`final_review.advance` also validates scope state; when `current_diff_hash`
differs from the stored hash, provide `current_changed_files` so the next review
iteration sees the current diff.
Session identifiers are bounded to 128 characters, and requested clean
iterations are bounded to 3-10 to cap assignment fanout.
The caller carries returned state between calls, while the coordinator stores a
project-scoped authoritative copy in its local SQLite state database. A new
stdio MCP process automatically resumes an exact caller-carried state, including
pending verifier and delta-risk assignments. Creation is insert-only and every
transition uses a durable revision compare-and-swap, so concurrent processes
cannot admit duplicate sessions or overwrite each other's progress. Unknown,
evicted, stale, or mutated state fails closed with sanitized expected/received
fingerprints and a restart, resume, or abandon recovery action; advancing a
completed session also fails. Each process retains at most 32 active sessions,
and durable storage is bounded independently.
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

- `new-subsystem`
- `unusually-broad-diff`

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
findings. The next `final_review.advance` call must keep the current diff hash,
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
malformed, or diff-changing decision calls. `ship` is rejected while any known
blocking finding remains and never substitutes for acceptance criteria or CI.
A valid `ship` decision is terminal for final review: it clears remaining
nonblocking lens work, returns `complete: true`, and schedules no reviewers.
The calling workflow must still satisfy the ticket's acceptance criteria and
confirm the latest pushed CI build is running or green before release or new
work. If that build failed, `ci-failure-follow-up` takes precedence and
requires exact diagnosis plus terminal success before release or new work.
`split` and `escalate` persist a contract-bound terminal hold, preserve
every completion blocker, schedule no reviewers, and reject any later advance
for that session.

## Finding Disposition And Escalation

Risk-planned review blocks only caused or worsened `CRITICAL`/`MAJOR` findings
with a concrete plausible major/critical security or human-safety impact in the
intended deployment and an in-scope changed remediation path. Incidental or
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
3. harness-aware defaults when Codex or Claude can be detected;
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

[final_review.models.codex]
pre_filter = "gpt-5.6-sol"
lens_review = "gpt-5.6-terra"
post_filter = "gpt-5.6-luna"
verifier = "gpt-5.6-sol"

[final_review.models.claude]
pre_filter = "opus"
lens_review = "sonnet"
post_filter = "haiku"
verifier = "opus"
```

Top-level phase values are harness-neutral. Optional `codex` or `claude`
tables override them one phase at a time for that harness. This lets a shared
repository use concrete Codex model IDs without routing Claude reviewers to
unsupported models.

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
strong-lens model. This keeps one source of truth for the Sol route and avoids
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
Rejected candidates do not become unresolved blockers; the iteration may count
as clean when no other blocking, malformed, or needs-human finding remains.
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
    "model_role": "gpt-5.6-terra",
    "fresh_context": true,
    "closed_after_result": true
  }
}
```

Append this object to every lens result before `final_review.advance`, and to a
verifier result after closing the verifier. Missing or mismatched attestations
block the transition.

## Protocol Versions

Initialization negotiates the requested MCP version. The server supports
`2024-11-05`, `2025-03-26`, `2025-06-18`, and `2025-11-25`; unsupported versions
receive a structured invalid-params response listing supported versions.

## Packaging

Marketplace installs require a packaged MCP binary for the host target. This
version packages static x86_64 and aarch64 Linux binaries plus macOS binaries
for both architectures. The launcher selects the matching artifact
deterministically. Source-tree Cargo
fallback is development-only and must be explicitly enabled by launcher
environment; if neither packaged binary nor approved fallback is available, MCP
enforcement is unavailable and must be reported as such. Incoming stdio requests
and conditional-lens fanout are bounded so malformed or bursty callers cannot
grow coordinator memory or review-agent count without limit. On any oversized
stdio frame, the server stops reading at the request byte limit, emits
`request_too_large`, and terminates so the harness can restart it; the process is
not reusable after that response. Each release binary embeds a fingerprint of
the Rust source, lockfile, and pinned Rust toolchain. Release checks verify that
fingerprint, checksums, and target executable formats.
