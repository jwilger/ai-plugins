---
name: development-workflow
description: Use when making repository changes, debugging, handling review feedback, verifying work, conducting final review, recovering CI, deciding the next development lifecycle step, or handling a missing or weaker final_review_protocol attestation or coordinator clean-iteration minimum.
---

# Development workflow

Use `workspace-reader.status` before choosing a workflow. Outside Git, without
configuration, or with invalid configuration, inspection remains available but
the plugin cannot provide configured workflow guidance. This advisory state
does not deny ordinary host mutation capabilities.

## Proportional and interruptible execution

Before expanding active scope for advisory or non-blocking feedback, run the
proportionality checkpoint in `workflow-rules.md` against the original goal,
acceptance criteria, trust boundaries, and completion claim. A review finding
does not authorize broader implementation. Keep materially required causal
work and mechanical gates fail-closed. Report distinct improvements and offer
the user a choice; worthwhile distinct work may enter the backlog without
expanding or delaying the active goal, while implementation requires the user
to explicitly expand the goal. A canonical final-review finding still makes
its iteration non-clean regardless of its backlog or report-only disposition.
In every checkpoint result, explicitly name all four boundaries and the outcome
for each; never substitute a bare reference to "the proportionality
checkpoint."

In every status update or answer about long-running work, explicitly report
four fields for each gate: (1) command or gate identity, (2) why it is required,
(3) progress or last-known state, and (4) cancellation path. Apply a user's
scope correction immediately to every subsequent action, promptly stop
optional work, report what stopped, preserve completed valid evidence, and do
not implicitly restart cancelled work. Reuse evidence only item by item when
its covered source or behavior, scope, baseline, configuration, environment,
inputs, and freshness contract are unchanged; record that decision and rerun
affected checks plus every required post-mutation or post-commit gate.
Deduplication never bypasses an acceptance criterion, final-review reset,
exact-revision gate, or CI-recovery hold.

## Per-edit durable checkpoints

After every individual implementation-file or test-file edit, stop and run the
smallest relevant test immediately. Bind the exact worktree snapshot and test
receipt to exactly one durable state, plus its next permitted action. Before
the first edit, commit, or push, resolve the full ticket-start commit OID and
persist it in this record; carry that immutable baseline through every later
checkpoint and into terminal review:

The available persistence owner is the active Tiber task's Git-backed notes,
not the unavailable native workflow scheduler. After every state transition,
append one single-line `checkpoint-v1` record with `tiber.note.add` (CLI:
`tiber note add`). The canonical wire form is the literal prefix
`checkpoint-v1 ` followed by one compact JSON object (no Markdown) with these
required keys: `baseline_oid`, `snapshot`, `state`, `test`, `gates`,
`delivery`, `ci`, and `next_action`. Use strings for scalar values and `null`
only where the following exact shapes permit it. `snapshot` is exactly
`{"head_oid":string,"tracked_sha256":string,"untracked_sha256":string}`.
`test` is either `null` or exactly
`{"command":string,"receipt_ref":string,"outcome":"pass"|"fail","failure_kind":string|null}`.
`gates` is exactly
`{"lightweight_review_receipt":string|null,"fast_gate_receipt":string|null,"exact_commit_verification_receipt":string|null}`.
`delivery` is either `null` or exactly
`{"mode":"local-only"|"direct-to-trunk"|"pull-request","commit_oid":string|null,"pushed_oid":string|null,"local_snapshot":string|null}`.
`ci` is exactly
`{"runs":[{"provider":string,"run_id":string,"commit_oid":string,"status":"queued"|"running"|"success"|"failure"}],"terminal_success_run_id":string|null}`.
The snapshot hashes are defined as follows:

- `tracked_sha256` is SHA-256 of the exact byte stream from
  `git diff --binary --full-index HEAD --`.
- `untracked_sha256` is SHA-256 of the byte stream produced by iterating
  `git ls-files --others --exclude-standard -z` in its emitted order and, for
  each path, appending its Git mode token (`100755` for an executable regular
  file, `100644` for another regular file, or `120000` for a symlink), one NUL
  byte, the path bytes, one NUL byte, the file's `git hash-object -- <path>`
  OID, and one newline byte. An unsupported file type is a recovery hold. The
  empty stream has the standard SHA-256 empty digest.

State-specific absent values remain `null` or empty arrays; never omit or
rename keys. JSON escaping is the only escaping. Before the first edit on a
clean task-start worktree, bootstrap the first record as
`pushed-or-delivery-mode-equivalent`: use the ticket-start OID for
`snapshot.head_oid`, set `test` to `null`, describe the already-authorized
starting identity in `delivery` (a pushed OID for a remote mode or
`local_snapshot` for local-only), and make the first planned causal edit the
sole `next_action`. This is the only delivered-state gate exception: all three
gate receipts are `null`, and CI may be empty, only when `baseline_oid` equals
`snapshot.head_oid`, both snapshot hashes prove the clean starting worktree,
and `delivery` identifies that same pre-existing baseline. A dirty,
non-baseline, or unreconciled starting worktree is a recovery hold, not a
bootstrap shortcut. Store bounded references rather than raw logs or secrets. At session start, restart, or handoff, read the task with
`tiber.show`, select its latest `checkpoint-v1`
record, and reconcile every identity with current Git and forge state before
acting. A malformed, missing, unpublished, or mismatched record is a fail-closed
recovery hold: do not edit, commit, push, or infer progress until the same task
record is reconciled. This note protocol records evidence only; it does not
emulate or claim native `workflow.*` enforcement.

- `failing`: commit and push are prohibited. Permit only the next causal edit
  needed to address that failure, reject unrelated or convenience changes, and
  immediately test again. `test` is required and `delivery` is `null`.
  Every gate receipt is `null`; CI entries may only describe already-delivered
  earlier commits and cannot satisfy a gate for this snapshot.
  A newly written test that passes unexpectedly is still `failing`, with an
  `invalid-test` reason and only the causal test rewrite as `next_action`; do
  not checkpoint that test as passing or introduce a fifth state.
- `passing-awaiting-gates-or-review`: freeze further implementation and test
  edits. Run the bounded lightweight review and repository fast pre-commit
  gate; any remediation is a new causal edit and therefore triggers another
  immediate focused test. `test` is required and `delivery` is `null`.
  Record each completed lightweight-review and fast-gate receipt as it occurs;
  exact-commit verification remains `null` until a commit exists.
- `committed`: record the signed commit OID and whether the next action is the
  delivery-mode checkpoint or a locally complete checkpoint; `delivery` is
  required and its commit OID must equal `snapshot.head_oid`. Lightweight-review
  and fast-gate receipts are required. Immediately after commit creation,
  `exact_commit_verification_receipt` may be `null` only while `next_action` is
  `verify-exact-commit`; append the next committed checkpoint after verification.
  A failed verification stays `committed`, records the bounded failure receipt,
  prohibits delivery, and permits only the causal repair action followed by
  another exact verification. Push or local completion requires a successful
  exact-commit verification receipt. CI for this new commit may still have no
  runs before remote delivery.
- `pushed-or-delivery-mode-equivalent`: record the exact pushed OID and CI runs,
  or the exact local-only terminal snapshot and the fact that remote mutation
  is unauthorized; `delivery` is required and must identify the exact
  state-appropriate commit or snapshot, and all three gate receipts remain
  required and successful except for the exact clean bootstrap above. Immediately after a
  successful remote push, `ci.runs` may be empty only while `next_action` is
  `register-exact-sha-ci-monitor`; append the next checkpoint as soon as the run
  reference exists. Otherwise direct-to-trunk and pull-request records require
  at least one CI run for the exact pushed OID; local-only requires no remote run. Set
  `terminal_success_run_id` only when it names an included exact-OID success
  run; queued, running, older-OID, or failed runs never satisfy readiness. When Tiber's opt-in final-review policy requires reviewed
  source and verification paths in a commit tree, the local equivalent is a
  required local commit; if commit authority is explicitly withheld, completion
  blocks without authorizing a push.

Every interruption or handoff must preserve the ticket-start baseline, state,
exact snapshot, test receipt, completed gates, and sole next permitted action.
Never infer progress from an unbound green test or a clean worktree alone.

When a test and its implementation cannot independently pass, declare one
bounded RED-to-GREEN pair before the implementation edit. Persist the expected
RED result and causal claim, allow only the paired implementation edit, test
immediately, and enter `passing-awaiting-gates-or-review` at GREEN. This is not
permission to bundle another behavior. The pair is not a fifth durable state:
its RED snapshot remains canonical `failing` with the paired implementation as
the sole next action until the immediate retest reaches GREEN.

Apply proportional checkpoints outside implementation and test files.
Documentation, configuration, and formatting-only changes run the smallest
relevant formatter, parser, validation, or executable example before their
lightweight review and delivery checkpoint. Generated output and mechanically
required metadata may accompany only the causal source snapshot that produces
or requires them. Never batch already-passing unrelated work for convenience.

The terminal full multi-lens review begins only after every planned increment
and acceptance criterion has reached its delivered checkpoint. It is not an
intermediate-commit gate, and lightweight review never replaces it. Review the
already-delivered identity: the pushed commit for direct-to-trunk or PR mode,
or the exact local snapshot for local-only. A clean unchanged terminal review
creates no commit, empty commit, push, or replacement local checkpoint.

Any terminal-review finding remediation must leave review and complete the
normal immediate-test, lightweight-review, and fast-gate checkpoint. Then use a
signed additive commit plus exact verification and authorized push for remote
modes, or a signed local commit only when required/authorized and otherwise a
new exact no-commit snapshot for local-only. Submit the diff-bound delta
assessment and rerun the complete selected lens set under the normal clean-streak
reset. Comprehensive, slow, integration, mutation, and pipeline-scale suites
remain in CI rather than being duplicated in a local exact-commit gate. A
completed exact-SHA CI failure immediately preempts review or other work through
the existing Tiber-owned recovery hold.

Keep checkpoint CI bounded per ref while continuing increments. Use existing
provider or repository coalescing/cancellation for superseded non-terminal runs
when available and authorized; otherwise wait for capacity before another push
would add obsolete queued work. Never cancel the current terminal candidate's
exact-SHA run, and never accept a successful older run as its readiness
evidence.

When explaining this workflow, explicitly state the RED pair's recorded causal
claim, comprehensive-suite ownership in CI with no duplicate comprehensive
local exact-commit gate, that every planned increment and acceptance criterion
must be checkpoint-delivered before terminal review begins, and the complete
terminal-remediation checkpoint. Do not leave any of them implied by a generic
reference to repository gates or final-review reset.

## Final-review protocol preflight

For every final-review path, complete this checklist before creating any risk
or review state:

1. Call `workspace-reader.status` through the same connected MCP that would run
   final review.
2. Require one `final_review_protocol` object with all three predicates:
   `contract_version >= 2`, `minimum_clean_iterations >= 3`, and
   `durable_pending_assignment_recovery: true`.
3. If any predicate is missing or weaker, classify the skill and connected MCP
   as stale or mismatched. Do not call `final_review.assess_risk` or
   `final_review.plan`; if a plan was already created, discard that entire
   session. State that zero clean iterations are accepted, launch no reviewers,
   and reject terminal delivery, pull/merge-request creation or update, merge,
   and readiness claims. Preserve separately authorized intermediate or
   terminal-finding-remediation checkpoints: their normal commit and push may
   proceed, but neither is terminal delivery or readiness evidence.
4. Recover by installing the current-host binaries from the updated marketplace
   checkout with `just install-development-system-binaries` (or
   `scripts/install-development-system-binaries.sh`), then rerun Development
   System setup for the current harness so the project-local MCP configuration
   is rewritten to the newly installed absolute binary path. Restart the
   harness. Start a new review session only after the same-MCP status call
   passes all three predicates; another plan call is never the runtime probe.
5. After planning, require the returned coordinator-owned
   `required_clean_iterations` to be at least the attested minimum. A lower
   value invalidates and discards that session; it cannot schedule reviewers or
   authorize terminal delivery. Reject terminal delivery, PR/MR creation or
   update, merge, and readiness until a fresh coordinator-compatible session
   succeeds. Preserve any separately authorized intermediate or
   terminal-finding-remediation commit/push checkpoint; it is not terminal
   delivery or readiness evidence.

When reporting a protocol mismatch, do not abbreviate this checklist. Name the
same-MCP status requirement and all three predicate names and thresholds; say
that `final_review.assess_risk` and `final_review.plan` are prohibited; say the
old session is discarded with zero accepted clean iterations and no reviewers;
name terminal delivery, pull/merge-request creation or update, merge, and
readiness as rejected actions while preserving separately authorized
intermediate or terminal-finding-remediation commit/push checkpoints; require current-host binary installation, rerunning
Development System setup for the current harness to rewrite the project-local
absolute MCP binding, and harness restart; and require a fresh plan whose
coordinator-owned minimum is at least the attested minimum. A generic
instruction to refresh, reconnect, or require
"three passes" is not a complete fail-closed recovery record.

For every workflow question, load [workflow
rules](references/workflow-rules.md). Before editing, classify artifact impact
relations and RED applicability. Keep diagnosis causal, bind readiness claims
to post-mutation exact-revision evidence, select review lenses from changed
system boundaries, and use the configured delivery mode.

Load the retained specialist contract only when its intent matches:

- Change classification or preflight: [change
  preflight](../../components/development-discipline/skills/change-preflight/SKILL.md).
- Causal debugging: [systematic
  debugging](../../components/development-discipline/skills/systematic-debugging/SKILL.md).
- RED-GREEN-refactor behavior: [test-driven
  development](../../components/development-discipline/skills/test-driven-development/SKILL.md).
- Exact-revision evidence: [verification before
  completion](../../components/development-discipline/skills/verification-before-completion/SKILL.md).
- Final-review scope, findings, or verifier flow: [final
  review](../../components/development-discipline/skills/final-review/SKILL.md).
- Worker or reviewer selection: [model
  routing](../../components/development-discipline/skills/model-routing/SKILL.md).
- Failed pushed CI: [CI failure
  follow-up](../../components/development-discipline/skills/ci-failure-follow-up/SKILL.md).
- Lifecycle-aware delivery: [delivery
  workflow](../../components/development-discipline/skills/delivery-workflow/SKILL.md).
- Commit-message rationale: [rationale commit
  messages](../../components/development-discipline/skills/rationale-commit-messages/SKILL.md).
- Authoring or reviewing a skill: [writing
  skills](../../components/development-discipline/skills/writing-skills/SKILL.md).

For every final-review path, keep these fail-closed invariants visible in the
working context rather than relying on a later summary. When asked how to run
or complete final review, state every applicable invariant before adjacent
delivery detail:

- Honor every explicit baseline, path scope, and read-only boundary. Otherwise,
  review the pinned baseline plus every unexcluded staged, unstaged, committed,
  renamed, mode-changed, and untracked byte. A review-only request reports
  findings and never edits; remediate valid findings only when remediation was
  requested.
- With no explicit base, use the immutable full ticket-start OID—never a newly
  resolved `origin/main`. For both base and explicitly uncommitted review, use
  the one-revision `git diff --find-renames --find-copies --end-of-options
<baseline-oid> --` content surface plus separately NUL-parse `git status
--short -z --untracked-files=all` as exact path bytes without display
  unquoting. Read in-scope untracked content, including newline-containing
  paths. Write the deduplicated inventory to a temporary NUL-delimited file,
  pass it with the full OID to `scripts/final-review-scope-hash.sh`, use exact
  helper stdout for plan `diff_hash` and every advance `current_diff_hash`, and
  delete the temporary file after each call. Reject triple-dot, index-only,
  bare-worktree, symbolic-baseline, and caller-invented hashes.
- Use a fresh-context subagent for every assigned lens in every iteration.
  Always include an independent `production-risk-footguns` lens for latent
  traps, fragile defaults, production-scale data/resource access, and
  burst/DOS-like load behavior; add other repository-agnostic lenses according
  to assessed risk.
- Every submitted canonical finding—including out-of-scope, report-only,
  already-known, defended, or later rejected feedback—makes that iteration
  non-clean, clears the clean streak and verified-clean receipts, and requires
  the complete selected lens set again in fresh contexts. Disposition controls
  remediation or reporting only; never carry unaffected peer evidence forward.
- Feed every defense or caller decision back to the relevant fresh lens
  reviewers. Never accept a defense only in the caller context. A defense
  cannot make its finding-bearing iteration clean; completion still requires
  three later consecutive complete finding-free iterations.
- When Tiber's opt-in final-review policy applies, explicitly require every
  fresh independent clean review and every substantive finding to be recorded
  through the durable `tiber.review.record` surface. Prose instructions,
  coordinator state, or an unrecorded reviewer result are not completion or
  delivery evidence.
- For pushed work, the most recently completed in-scope build must be green.
  A queued or running build is only waiting, never replacement evidence, and a
  completed failed required job creates the CI-recovery hold.
- When exceptional risk is at issue, explicitly return this complete checklist;
  never summarize an item away:
  1. Reject an empty, unknown, or invented exceptional trigger. The only
     supported IDs are `destructive-or-irreversible-operation`,
     `authentication-or-authorization-boundary`,
     `sensitive-data-migration`, `cryptographic-behavior`, and
     `safety-critical-behavior`.
  2. Require both a supported trigger and an explicitly exceptional dimension
     for an exceptional overall profile.
  3. Keep supported triggers recorded on a lower profile when concrete
     mitigations keep risk below exceptional.
  4. Give the extra independent discovery sample only to dimensions explicitly
     rated exceptional.
  5. Validate every later delta with the same supported-trigger and
     explicitly-exceptional-dimension rules.
  6. For a legitimate material-delta reassessment, merge and preserve trigger
     evidence in authoritative state, clear the clean streak and verified-clean
     receipts, invalidate every old peer result, and require three fresh,
     consecutive, complete, finding-free iterations of the entire selected
     lens set.

When a `final_review.plan` handoff is truncated, recover it instead of
replanning. Call `final_review.pending_assignments` with the retained
`state_ref`; its compact summary is authoritative for every pending lens,
verifier, or delta-risk assignment, including the exact `subagent_key`, exact
`model_role`, phase metadata, close policy, result-schema version, and the
phase-specific assignment identity and evidence references. Pass one exact
`subagent_key` back to that tool to retrieve only that assignment's original
durable prompt and schema. An incomplete `final_review.resume_latest` returns
the same compact summary. Supply `work_item_id` when the original review was
ticket-bound, and never mix a session, project root, work item, or fingerprint
from different handoffs. These retrievals are read-only and idempotent: they
append no event, change no revision, fingerprint, scope, or hash, and never
reassign a lens or role. If a pre-upgrade pending verifier or delta-risk record
lacks recoverable assignment facts, follow its explicit
`recovery=restart_final_review` guidance; never reconstruct it from partial
projection state. A missing or invalid lifecycle/model attestation is malformed
assigned evidence: accept the coordinator's non-clean reset transition, discard
the full iteration, and execute every fresh next-iteration assignment. Never
repair the old assignment or preserve peer results from that invalidated
iteration. Apply the same rule to malformed verifier evidence and invalid
verifier assignment provenance or verdict coverage: close the pending verifier
and rerun the full selected lens set returned by the reset transition.

The pinned ticket-start Git baseline plus in-repository paths, contents, modes, and untracked
inventory define reviewed source scope. Coordinator events, snapshots, state
files, and projections are bookkeeping outside that hash. After terminal source
review, a content-identical commit does not reopen review merely because HEAD,
staging partition, signature, or commit metadata changed. Real path, content,
mode, untracked-content, pinned-baseline, or requested-scope changes do reopen
it. The terminal sequence is delivered checkpoint, full review of that exact
pushed commit or local snapshot, then readiness evidence. A clean unchanged
review adds review evidence only. Pushed readiness requires terminal-success CI
for the exact final-reviewed SHA; pending CI is
`review-complete-awaiting-exact-sha-ci`. Source-changing remediation creates a
new mode-specific checkpoint and resets review as described above.

Load `delivery-workflow` before the first implementation or test increment so
the configured delivery mode and green-increment preservation cadence are known
before even a standalone test edit, commit, or push. After final review, recheck it only to confirm that
the reviewed delivered identity has the required exact-SHA CI or local
readiness evidence; a clean unchanged review creates no final delivery action.

## Structured lifecycle

The bootstrap plugin exposes structured `final_review.*` coordination, but not
the native `workflow.*`, `workspace-editor.*`, or `project-runner.*` services.
Use ordinary host tools for authorized repository changes and verification, and
follow the RED-GREEN-refactor, exact-evidence, review, and delivery guidance in
the specialist contracts above. Do not claim that plugin instructions, hooks,
or final-review records enforce harness capabilities or authorize external
actions.

The retained source-level native lifecycle is for the planned standalone Tiber
scheduler. It is not bound to the currently installed Tiber task-board binary.
Once that separate scheduler binding ships, assignment-bound editor and runner
receipts support the mechanical RED, GREEN, verification, review, and delivery
transitions: `workflow.record_red`,
`workflow.authorize_implementation`, `workflow.record_green`,
`workflow.begin_verification`, `workflow.record_verification`,
`workflow.record_clean_review`, `workflow.authorize_delivery`, and
`workflow.complete_delivery`. Until then, do not call or emulate those native
transitions through the advisory bootstrap surface. Installed Tiber task
transitions enforce only the separate opt-in
`[final_review].minimum_clean_reviews` completion and trailer-delivery policy;
they do not enforce RED, GREEN, verification, editor, runner, or
complete-delivery transitions.

Before a new task, inspect forge CI. Require a successful completed run for the
candidate revision; queued/running is not terminal evidence, and any completed
failed required job activates a repository-wide hold. Use Tiber's single
CI-recovery owner and record SHA, run, checks, and terminal status. Development
Discipline reads that Tiber hold when gating delivery; it does not own one.
