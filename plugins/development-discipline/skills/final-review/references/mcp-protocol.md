# Final Review MCP Protocol

Read this reference only when running, debugging, or changing the MCP-enforced
final-review path.

## Required Scope

`final_review.plan` requires a complete reviewed `changed_files` inventory and a
non-placeholder `diff_hash`. Before the ticket's first commit or push, resolve
and retain the full `baseline_commit` OID. Incremental pushes can move the named
base past the ticket, so final review must not re-resolve that ref. Supply the
same full OID to the scope-hash helper, `final_review.assess_risk`, and the
risk-planned `final_review.plan`; missing, symbolic, abbreviated, or changed
baseline values fail closed. Compute both `diff_hash` and every
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
The caller carries returned state between calls, but the stdio MCP process owns
the authoritative copy. Keep one process alive for the full review. Unknown,
evicted, stale, or mutated session state fails closed; advancing a completed
session also fails. The coordinator retains at most 32 active sessions with LRU
eviction.
When an advance returns `verifier_required`, the server retains the pending
assignment ID and exact pre-verifier arguments. Until the caller resubmits those
same arguments plus the matching `verifier_result`, missing verification or
changed lens/scope/decision arguments fail closed. Pending verifier state is
cleared only by an accepted advanced transition or session eviction.
Call `final_review.filter_findings` before the initial advance, prepare any
applicable `caller_decisions` from its retained findings, and include them on
that first advance. Once `verifier_required` is returned, resubmission may add
only `verifier_result`; introducing a defense or accepted-risk decision at that
point changes the frozen arguments and fails closed.
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

## Unrelated Findings And Security Escalation

Pass a one-time `unrelated_finding_policy` to `final_review.plan` when the
user chooses routing by lens or review severity. The MCP resolves each
out-of-scope finding with this precedence: `by_lens`, then `by_severity`, then
`default`. Valid dispositions are `address-now`, `follow-up-ticket`, and
`report`. An `address-now` observation becomes a human decision; a
`follow-up-ticket` observation requires a matching `unrelated_follow_ups`
entry on `final_review.advance` with its finding ID, lens, and nonblank ticket
reference. `report` stays non-blocking and is retained in bounded review state.

Review severity is exactly one of `CRITICAL`, `MAJOR`, `MINOR`, or `TRIVIAL` and is separate from
`security_impact` (`none`, `minor`, `moderate`, `major`, `critical`). Any
suspected PII exposure or major/critical security observation requires a
matching `security_escalations` entry with a nonblank high-priority ticket
reference before advance. An out-of-scope observation cannot claim an
unverified current-ticket fix; if it truly blocks the ticket, classify it as
relevant and address it in the ticket instead.

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
pre_filter = "cheap-fast-filter"
lens_review = "strong-reviewer"
post_filter = "cheap-fast-filter"
verifier = "cheap-fast-verifier"

[final_review.models.codex]
pre_filter = "gpt-5.6-luna"
lens_review = "gpt-5.6-terra"
post_filter = "gpt-5.6-luna"
verifier = "gpt-5.6-sol"
```

Top-level phase values are harness-neutral. Optional `codex` or `claude`
tables override them one phase at a time for that harness. This lets a shared
repository use concrete Codex model IDs without routing Claude reviewers to
unsupported models.

Projects can also add `[final_review.dispositions.<SEVERITY>]` tables for
`CRITICAL`, `MAJOR`, `MINOR`, and `TRIVIAL`. Each table must map every active
review lens to exactly one of `block`, `ticket`, `document`, or `ignore`.
Incomplete, unknown, or invalid matrix entries fail closed; without a matrix,
all severity-and-lens combinations default to `block`.

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

- `pre_filter` is optional model assistance for a large or noisy scope. It may
  focus context but must never skip a review lens.
- `lens_review` is one MCP-assigned caller subagent for every lens and
  iteration.
- `post_filter` is the deterministic `final_review.filter_findings` path by
  default, so its model label is normally not invoked.
- `verifier` is one conditional batched caller subagent when post-filtering
  leaves actionable or needs-human-decision candidates. A missing verifier
  result blocks the transition. A failed verifier retains every candidate.

When `final_review.advance` returns `transition_status: verifier_required`, run
the returned assignment with its exact `subagent_key` and `model_role`, close
the subagent after collecting its result, then resubmit the same lens results
with `verifier_result`. Verified results require exactly one `confirmed`,
`rejected`, or `uncertain` verdict per candidate, a final review severity, and
a non-empty rationale. The server records reviewer and verifier severities and
routes with the verifier's final severity. Rejected candidates do not
become unresolved blockers, but the iteration remains non-clean because a lens
raised a finding. Uncertain candidates stay open for human decision.

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
