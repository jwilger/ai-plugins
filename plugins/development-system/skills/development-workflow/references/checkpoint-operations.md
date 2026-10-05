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

| Operation                     | Additional fields                                                | Recorded transition                                                                                                                                                                                                                                                             |
| ----------------------------- | ---------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `initialize`                  | `mode`, `causal_edit`; remote modes also require `remote`, `ref` | Clean current HEAD becomes the immutable baseline. Modes are `local-only`, `direct-to-trunk`, or `pull-request`. Remote baseline identity is read with `git ls-remote`; `ref` must be a full branch ref.                                                                        |
| `edit-pass`                   | Evidence                                                         | Pending causal edit or invalid-test rewrite becomes passing, awaiting fresh lightweight review.                                                                                                                                                                                 |
| `edit-fail`                   | Evidence, `failure_kind`, `causal_repair`                        | Failed focused test clears every gate and permits only its causal edit.                                                                                                                                                                                                         |
| `edit-invalid-test`           | Evidence, `causal_repair`                                        | Unexpectedly passing RED preserves outcome `pass` with failure kind `invalid-test` and requires a test rewrite.                                                                                                                                                                 |
| `lightweight-review-pass`     | Evidence, `route`                                                | `route: "commit"` requires the normal commit through its actual pre-commit hook; `route: "local-snapshot"` requires a standalone fast gate for local-only delivery.                                                                                                             |
| `lightweight-review-fail`     | Evidence, `causal_repair`                                        | Review findings clear gates and permit only causal repair.                                                                                                                                                                                                                      |
| `fast-gate-pass`              | Evidence                                                         | Records the standalone local-snapshot fast gate. Do not use it to duplicate a gate owned by the commit hook.                                                                                                                                                                    |
| `fast-gate-fail`              | Evidence, `causal_repair`                                        | Clears gates and records the actual failed standalone gate.                                                                                                                                                                                                                     |
| `hook-failure`                | Evidence, `causal_repair`                                        | Records a failed pre-commit attempt with unchanged HEAD; staged, unstaged, newly staged, and hook-modified content remain intact.                                                                                                                                               |
| `commit-success`              | Evidence, `mode`                                                 | Requires an actual new direct successor commit containing exactly the retained reviewed source. Supply actual successful commit/hook output, never a standalone gate receipt represented as a hook. Records the commit and hook evidence together, awaiting exact verification. |
| `exact-verify-pass`           | Evidence                                                         | Records actual source, message, and signature verification of the current commit; permits mode-specific delivery.                                                                                                                                                               |
| `exact-verify-fail`           | Evidence                                                         | Records failed exact verification and requires its causal repair.                                                                                                                                                                                                               |
| `exact-verify-retry`          | Evidence                                                         | Records the completed causal repair and requests fresh exact verification. A metadata-only amended commit is accepted if source and ancestry are unchanged; source changes remain a recovery hold.                                                                              |
| `local-delivery`              | None                                                             | Records the already verified local commit as delivered.                                                                                                                                                                                                                         |
| `local-snapshot-delivery`     | Evidence                                                         | Records exact verification and delivery of the reviewed local-only snapshot after its fast gate. No commit receipt is invented.                                                                                                                                                 |
| `push-readback`               | `remote`, `ref`                                                  | Reads the authoritative full branch ref with `git ls-remote` and requires the verified commit OID. The caller must already have completed its authorized push.                                                                                                                  |
| `ci-register`                 | Evidence, `provider`, `run_id`, `commit_oid`, `status`           | Appends a new run for the exact pushed SHA. Status is `queued`, `running`, `success`, or `failure`.                                                                                                                                                                             |
| `ci-observe`                  | Same as `ci-register`                                            | Appends an observation of a registered nonterminal run. Terminal results cannot be overwritten; retries register new run IDs.                                                                                                                                                   |
| `ci-recovery`                 | Evidence, `causal_repair`                                        | Failed required CI enters causal remediation and clears current gate credits while retaining all CI observations.                                                                                                                                                               |
| `terminal-review-pass`        | Evidence                                                         | Records actual terminal review of the unchanged delivered identity and sets `next_action: "complete"`.                                                                                                                                                                          |
| `terminal-review-remediation` | Evidence, `causal_repair`                                        | Records terminal findings before any repair; clears gates and delivery and permits only the causal edit.                                                                                                                                                                        |
| `read`                        | Input is exactly `{}`; no CAS fields                             | Returns the retained receipt for `OPERATION_ID`, without advancing or claiming the current source still matches it.                                                                                                                                                             |

Successful stdout is a compact JSON operation receipt containing the operation
ID/name, request digest, complete resulting record, exact record digest,
source identity, delivery mode, and bounded evidence reference. Store it for
handoff. An identical retry returns that original receipt even after newer
operations have progressed; different input under the same ID is rejected.
Read the authoritative `.latest` separately before deciding the next action.
An old receipt is historical evidence, not permission to repeat an external
commit or push.

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
history. A clean terminal review requires that successful result in remote
modes. Source-changing remediation requires fresh focused testing, review,
hook/fast gate, exact verification, delivery, CI, and terminal review.

The public operations preserve the `checkpoint-v1` wire schema. The low-level
`write-local-checkpoint.sh` and `record-checkpoint-failure.sh` remain compatible
for existing retained records. Full-record proposals are a compatibility
surface, not the ordinary workflow API. Successful typed commits need a retained
typed review receipt so source can be compared independently of staging and
HEAD; reconcile a legacy pending commit explicitly instead of inventing that
missing source attestation.
