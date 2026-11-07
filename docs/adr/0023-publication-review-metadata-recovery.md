# ADR-0023: Recover unpublished metadata review failures through typed checkpoints

## Status

Accepted

## Date

2026-11-07

## Context

A publication review can reject commit metadata after exact verification has
passed. The checkpoint then awaits `increment-review`. Existing exact failure
operations accept only `verify-exact-commit`, while causal source remediation
requires the original HEAD and discards source-bound test/hook credit. A mistaken
source-identical amendment leaves neither route available. Resetting checkpoints,
fabricating a gate failure, or changing source solely to navigate the state machine
would destroy the relationship between evidence and the action performed.

## Decision

Add one typed operation, `increment-review-metadata-failure`, using the existing
lock, CAS, derivation, receipt, and crash-recovery machinery. Require an explicit
failed independent review whose findings are all commit metadata, bound to the
original candidate. Accept only verified direct-to-trunk predecessors with clean
raw source and typed source evidence. Verify original/current commits against
explicit exact tree and sole-parent identities, and preserve the task baseline,
tests, hook evidence, CI observations, and diagnoses.

Bind the readback to the current named trunk branch and configured upstream. Read
the actual single push destination and require its target branch to equal the
shared sole parent. Retain the observed authority and URL digest. Reject embedded
URL credentials before contacting the remote. This proves current target-ref
ancestry only, not absence of historical publication or other refs. Caller-known
publication remains disqualifying. Legacy operation receipts lack remote authority,
so this explicit configured-upstream binding is necessary for existing records.

Record failed identity verification, clear publication/final-review credit, and
enter the existing exact-identity repair route. Normal metadata amendment, fresh
source/message/signature verification, and independent publication review remain
required. The helper performs no commit, reset, push, fetch, or source edit.

## Consequences

The state machine can represent an actual review failure without inventing tests
or losing history. The operation is deliberately unavailable for merge commits,
stacked unpublished increments, source-changing repairs, unknown authority, and
published candidates. Deterministic CLI tests exercise acceptance, refusal,
identity drift, source filters, history, and replay; no model-backed evaluation is
needed for this mechanical transition.

Security risk centers on false publication authority and evidence laundering.
Strict source/ancestry checks, configured authority, explicit failed evidence, and
fresh downstream gates bound that risk within the existing trusted-owner model.
Review independence remains caller-supplied evidence, not cryptographic attestation.
There is no direct human or physical safety effect.

## Alternatives considered

- Reuse causal source remediation: misstates a metadata defect and cannot reconcile
  an already-amended HEAD without losing source-bound history.
- Relax `exact-verify-fail`: broadens unrelated routes and cannot distinguish an
  independent publication review failure from an exact-verification result.
- Reset HEAD or reconstruct checkpoints manually: discards or fabricates history
  instead of representing the observed event.
- Build general ancestry/publication discovery: exceeds this bounded recovery and
  would need historical authority unavailable in existing receipts.

## Revisit when

Real recovery cases require stacked commits, merges, or historical publication
attestation, or typed initialization persists durable remote authority that can
replace the configured-upstream binding.
