# Typed local checkpoint operations

Use the public operation script from the target repository. It records actions
that have already happened; it never runs tests, review, commit hooks, commits,
repairs, or pushes on the caller's behalf.

```shell
"$plugin_root/scripts/transition-local-checkpoint.sh" \
  CHECKPOINT_ID OPERATION_ID OPERATION /absolute/path/to/input.json
```

Keep input and evidence files outside the worktree. Choose an operation ID once
for one attempted transition and retain it with the exact request for retries.
Every operation except `read` takes `expected_generation` (the successor's
integer generation) and `expected_predecessor` (SHA-256 of the exact current
newline-terminated `.latest` bytes). Initialization uses `0` and JSON `null`.
Never construct the successor record yourself. The writer derives it from the
validated authoritative predecessor while holding the same exclusive task lock
used by the compatibility writer and failure-recovery helper.

For example, after an authorized causal edit and its actual focused test:

```json
{
  "expected_generation": 1,
  "expected_predecessor": "<64 lowercase hexadecimal characters>",
  "command": "the actual focused test command",
  "receipt_file": "/absolute/path/to/retained-test-output"
}
```

Pass that file with `edit-pass` or use `edit-fail` and add `failure_kind` and
`causal_repair`. The helper hashes current Git source identity, preserves the
immutable baseline, and derives gates, delivery, CI history, and `next_action`.
Unknown or missing fields are rejected. All operation-specific values in the
following table are nonblank strings. “Evidence” means the two fields `command`
and `receipt_file`; the file must be readable, nonempty, and outside the worktree.

| Operation                           | Additional fields                                                                                     | Recorded transition                                                                                                                                                                                                                                                                                                                                                                                           |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `initialize`                        | `mode`, `causal_edit`; remote modes also require `remote`, `ref`                                      | Clean current HEAD becomes the immutable baseline. Modes are `local-only`, `direct-to-trunk`, or `pull-request`. Remote baseline identity is read with `git ls-remote`; `ref` must be a full branch ref.                                                                                                                                                                                                      |
| `begin-edit`                        | Evidence, `causal_edit`                                                                               | Starts the next approved increment from unchanged verified and delivered source, including while exact-SHA CI is queued/running. Retains the baseline and CI observations, clears prior gate/delivery/current-CI credits, and awaits fresh edit results. A failed-CI recovery hold requires `ci-recovery`.                                                                                                    |
| `edit-pass`                         | Evidence                                                                                              | Pending causal edit or invalid-test rewrite becomes passing, awaiting fresh lightweight review.                                                                                                                                                                                                                                                                                                               |
| `edit-fail`                         | Evidence, `failure_kind`, `causal_repair`                                                             | Failed focused test clears every gate and permits only its causal edit.                                                                                                                                                                                                                                                                                                                                       |
| `edit-invalid-test`                 | Evidence, `causal_repair`                                                                             | Unexpectedly passing RED preserves outcome `pass` with failure kind `invalid-test` and requires a test rewrite.                                                                                                                                                                                                                                                                                               |
| `lightweight-review-pass`           | Evidence, `route`                                                                                     | `route: "commit"` requires the normal commit through its actual pre-commit hook; `route: "local-snapshot"` requires a standalone fast gate for local-only delivery.                                                                                                                                                                                                                                           |
| `lightweight-review-fail`           | Evidence, `causal_repair`                                                                             | Review findings clear gates and permit only causal repair.                                                                                                                                                                                                                                                                                                                                                    |
| `fast-gate-pass`                    | Evidence                                                                                              | Records the standalone local-snapshot fast gate. Do not use it to duplicate a gate owned by the commit hook.                                                                                                                                                                                                                                                                                                  |
| `fast-gate-fail`                    | Evidence, `causal_repair`                                                                             | Clears gates and records the actual failed standalone gate.                                                                                                                                                                                                                                                                                                                                                   |
| `hook-failure`                      | Evidence, `causal_repair`                                                                             | Records a failed pre-commit attempt with unchanged HEAD; staged, unstaged, newly staged, and hook-modified content remain intact.                                                                                                                                                                                                                                                                             |
| `commit-success`                    | Evidence, `mode`                                                                                      | Requires an actual new direct successor commit containing exactly the retained reviewed source. Supply actual successful commit/hook output, never a standalone gate receipt represented as a hook. Records the commit and hook evidence together, awaiting exact verification. Explicit `mode` may rebind `direct-to-trunk` and `pull-request`; crossing between local and remote gate families is rejected. |
| `exact-verify-pass`                 | Evidence                                                                                              | Records actual source, message, and signature verification of the current commit; permits mode-specific delivery.                                                                                                                                                                                                                                                                                             |
| `exact-verify-fail`                 | Evidence                                                                                              | Records failed exact verification and requires its causal repair.                                                                                                                                                                                                                                                                                                                                             |
| `exact-verify-retry`                | Evidence                                                                                              | Records the completed causal repair and requests fresh exact verification. A metadata-only amended commit is accepted if source and ancestry are unchanged; source changes remain a recovery hold.                                                                                                                                                                                                            |
| `local-delivery`                    | None                                                                                                  | Records the already verified local commit as delivered.                                                                                                                                                                                                                                                                                                                                                       |
| `local-snapshot-delivery`           | Evidence                                                                                              | Records exact verification and delivery of the reviewed local-only snapshot after its fast gate. No commit receipt is invented.                                                                                                                                                                                                                                                                               |
| `push-readback`                     | `remote`, `ref`                                                                                       | Reads the authoritative full branch ref with `git ls-remote` and requires the verified commit OID. The caller must already have completed its authorized push.                                                                                                                                                                                                                                                |
| `ci-register`                       | Evidence, `provider`, `run_id`, `commit_oid`, `status`                                                | Appends a new run for the exact pushed SHA. Status is `queued`, `running`, `success`, or `failure`.                                                                                                                                                                                                                                                                                                           |
| `ci-observe`                        | Same as `ci-register`                                                                                 | Appends an observation of a registered nonterminal run. Terminal results cannot be overwritten; retries register new run IDs.                                                                                                                                                                                                                                                                                 |
| `ci-retry-diagnosis`                | Evidence, `provider`, `run_id`, `commit_oid`, `classification`, `rationale`                           | Records a concrete `transient` or `unrelated` diagnosis for an existing failed run before retry registration. It preserves all prior observations and diagnoses and supplies no success credit.                                                                                                                                                                                                               |
| `ci-recovery`                       | Evidence, `causal_repair`                                                                             | Failed required CI enters causal remediation and clears current gate credits while retaining all CI observations.                                                                                                                                                                                                                                                                                             |
| `increment-review-metadata-failure` | Evidence, `review_kind`, `original_oid`, `replacement_oid`, `tree_oid`, `parent_oid`, `remote`, `ref` | Records an actual failed independent metadata review from `increment-review`, reconciles an unchanged or source-identical sibling HEAD, and enters `repair-exact-identity-verification`. See the bounded recovery contract below.                                                                                                                                                                             |
| `increment-review-pass`             | Evidence                                                                                              | Records proportional independent review bound to the verified trunk increment, exact source and immutable task baseline; permits `push` without certifying final acceptance. A legacy committed `terminal-review` checkpoint must explicitly use this operation.                                                                                                                                              |
| `terminal-review-pass`              | Evidence                                                                                              | Records final acceptance of the unchanged delivered identity after required exact-SHA CI; persists a separate snapshot/baseline-bound final review receipt and permits `complete`. Increment publication review never substitutes for this operation.                                                                                                                                                         |
| `terminal-review-remediation`       | Evidence, `causal_repair`                                                                             | Records terminal findings before any repair; clears gates and delivery and permits only the causal edit.                                                                                                                                                                                                                                                                                                      |
| `read`                              | Input is exactly `{}`; no CAS fields                                                                  | Returns the retained receipt for `OPERATION_ID`, without advancing or claiming the current source still matches it.                                                                                                                                                                                                                                                                                           |

Successful stdout is a compact JSON operation receipt containing the operation
ID/name, request digest, complete resulting record, exact record digest,
source identity, delivery mode, and bounded evidence reference. Store it for
handoff. An identical retry returns that original receipt even after newer
operations have progressed; different input under the same ID is rejected.
Read the authoritative `.latest` separately before deciding the next action.
An old receipt is historical evidence, not permission to repeat an external
commit or push.

### Publication review metadata recovery

Use `increment-review-metadata-failure` only for an actual independent publication
review that failed solely on commit metadata. Set `review_kind` to
`independent-publication` and retain the actual JSON review as `receipt_file`:

```json
{
  "verdict": "NEEDS_CHANGES",
  "candidate": "<original full commit OID>",
  "findings": [
    {
      "scope": "commit metadata",
      "title": "Required subject format is missing"
    }
  ]
}
```

Additional report fields are retained through the complete file digest. Every
finding must have that scope and a nonblank title. This validates the caller's
evidence declaration; it does not independently attest the reviewer or turn a
self-review into independent review. Never manufacture a report to pass this gate.

The predecessor must be a verified, committed direct-to-trunk candidate awaiting
`increment-review`, with its typed raw-source receipt. `original_oid` must match
both retained snapshot and delivery commit. `replacement_oid` must be current HEAD;
it may equal the original or identify an already-created sibling amendment. Both
commits must have exactly `tree_oid` and the same sole `parent_oid`. The index,
worktree, untracked source, and committed submodules must be clean, and raw source
must match the predecessor, including changes hidden by clean filters.

The named current branch must equal the full `ref`, and its configured upstream
must equal `remote`/`ref`. The remote must have exactly one push destination with
no embedded URL credentials. Readback of that actual push destination must show
the sole parent at `ref`; neither candidate may have retained publication CI.
This is point-in-time evidence that neither candidate is in the target branch's
current ancestry. It does not prove absence of historical publication or other
refs. Known published commits must not use this recovery. Unknown, moved,
unavailable, or mismatched authority remains a hold; do not reset or change an
upstream to force acceptance. Older receipts did not persist remote authority,
so this operation explicitly binds and retains the configured upstream readback.

The new operation receipt adds `metadata_recovery` with original/replacement,
tree/parent, remote/ref, observed remote OID, review kind, and a SHA-256 digest of
the push URL (never the URL). The immutable task baseline, prior test and hook
evidence, and all CI observations/diagnoses remain. Exact verification is recorded
as failed; increment/final review and terminal CI success credits are cleared.
The live replacement becomes the delivery commit without earning publication credit.

After this transition, repair the metadata through normal signed hook-backed
amendment as required by repository policy. Then `exact-verify-retry` records the
completed repair and requests fresh exact source/message/signature verification.
Run that verification before `exact-verify-pass`, followed by a fresh independent
`increment-review-pass` before push. The recovery itself never amends, verifies,
signs, pushes, or substitutes retained source-bound hook evidence for new identity
checks. Source-changing findings use normal causal remediation instead.

This transition is available only through the typed API; compatibility full-record
proposals cannot mint it. The ordinary CAS, immutable receipt, replay, and crash
recovery rules apply.

Before changing source for the next planned or causal increment, call `begin-edit`
with the actual retained planning/finding evidence and its specific `causal_edit`.
The predecessor must still match the exact verified local committed or delivered snapshot. The
result is `awaiting-causal-edit`; it makes no new test-success or failure claim.
Then perform the edit and record `edit-pass` or `edit-fail`, followed by every
fresh required gate. A pending CI run remains in history and earns no credit for
the next commit. If the predecessor already records failed CI, call `ci-recovery`
with the actual failure evidence instead. Source or HEAD drift requires
reconciliation before `begin-edit`; it cannot retroactively authorize edits.

Operation receipts live beside `.latest` in an owner-only `.latest.operations/`
directory. The writer durably prepares `.latest.pending-operation` before its
atomic publication. After a crash, the next invocation of either writer entry
point reconciles that intent against exact predecessor/successor bytes. If the
rename happened, it finishes the receipt without repeating the transition; if
it did not, it allows a fresh CAS attempt. Unexplained identity conflicts fail
closed. Do not edit or delete these files to work around a failure.

Tests, review, hook execution, signatures, and CI truth are evidence supplied by
the caller, not independently attested by a log path. Retain actual evidence
and reconcile it before calling. A CI `success` observation must represent the
authoritative terminal required-build result for that exact SHA, not one green
job or a queued retry. Earlier failed attempts remain in the append-only
history. Proportional independent increment review precedes each trunk push.
Remote completion requires separate cumulative final acceptance and exact-SHA
terminal CI success after publication. Source-changing remediation requires
fresh focused testing, affected independent increment review, hook/fast gate,
exact verification, delivery, CI, and renewed affected final review.

The public operations preserve the `checkpoint-v1` record fields and add the
`awaiting-causal-edit` state for a new increment without invented test outcomes.
Use the version 6.10 or newer helper and validator for that state; older
validators may reject it. The low-level
`write-local-checkpoint.sh` and `record-checkpoint-failure.sh` remain compatible
for existing retained records. Full-record proposals are a compatibility
surface, not the ordinary workflow API. Successful typed commits need a retained
typed review receipt so source can be compared independently of staging and
HEAD; reconcile a legacy pending commit explicitly instead of inventing that
missing source attestation.

Compatibility gate failures with an exact typed predecessor use the same typed
failure operation, task lock, CAS checks, and durable publication recovery. The
compatibility helper derives a stable operation ID from the expected generation
and predecessor; retry with the same arguments to recover interrupted
publication. Changed arguments under that ID fail closed. Both hook and
lightweight-review failures retain the established delivery mode, including
local-only. Remote-to-remote rebinding remains available at `commit-success`;
a failure cannot authorize crossing between remote and local gate families.
Legacy records still support compatibility failure publication. If their current
record and exact receipt do not establish a delivery mode, typed continuation
holds: continue through the compatibility full-record workflow while explicitly
reconciling delivery and review evidence. Do not infer local-only from absent CI,
reuse an unrelated historical receipt, or manufacture missing source evidence.

The published `checkpoint-v1` snapshot retains Git's path-aware text/clean-filter
conversion for untracked regular files and raw symlink targets. The separate
reviewed-source identity hashes raw bytes, executable modes, and exact path
bytes, independently of filters, HEAD, or staging. Thus a raw edit that cleans to
the same Git blob still invalidates review. Raw identity is checked again before
the writer creates its durable publication intent. Regular source files are hashed in
64 KiB chunks with the repository's Git object format and checked for detected
type, replacement, size, or metadata changes during reading. Path inventories
and Git diff output remain buffered; this is a bound on file-content allocation,
not a constant-memory guarantee for arbitrarily large repositories.

An index-mode `160000` gitlink binds source identity to the actual HEAD of the
clean child repository rooted at that exact path. Parent staging does not change
that identity. A clean baseline or successful committed verification, delivery,
or readiness claim additionally requires every gitlink's actual child HEAD and
index entry to match the authoritative commit tree, regardless of submodule
ignore settings. Staging a reviewed child reference does not commit it. These
checks run again before publication; failure recording remains available for
causal recovery. Dirty or untracked child content, including nested submodule
changes despite ignore settings, requires reconciliation before checkpointing.
Missing or deinitialized submodules also hold: initialize them at their recorded
paths before continuing. Ordinary directories are not silently treated as
submodules or omitted from source identity.

CI observations bind an already registered provider/run/commit and may arrive
while the next increment is in progress. Historical success adds evidence without
crediting the current candidate. A failed required run preempts work with
`enter-ci-recovery`. For a change-caused failure, use `ci-recovery` and fresh causal
repair gates. For a proven transient or unrelated failure with unchanged source,
use `ci-retry-diagnosis` with `provider`, `run_id`, `commit_oid`, `classification`
(`transient` or `unrelated`), concrete `rationale`, and the normal evidence fields.
The diagnosis is append-only and binds the recorded failed run; it permits a fresh
registered retry and never fabricates CI success. Undiagnosed retries cannot clear
failure holds. Terminal findings may enter `terminal-review-remediation` while CI
registration or observation is pending; a failed CI hold takes precedence.

An observation for a newer current-SHA run invalidates older success credit.
Unresolved failures in retained history preempt readiness; a historical success
for another SHA adds evidence without crediting the current candidate. CI
observation never completes terminal review. After the recorded final-review contract completes its required independent
coverage and adjudication, and required CI succeeds for the unchanged delivered
SHA, record the actual review evidence with
`terminal-review-pass`. Only that separate transition sets `complete`.
