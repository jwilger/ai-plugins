# ADR 0017: Recoverable, evidence-bound review orchestration

## Status

Accepted

## Context

Independent reviews continue to find real defects, including defects missed in
previous rounds. Persistence failures and manual lifecycle bookkeeping obscure
that value. A fixed review timer can also strand required repairs and fresh
reviews after the allotted interval.

Isolated native reproduction showed Git object creation failing when only the
review replica directory was writable. Granting the required Git object and
advisory-ref paths allowed the identical request to succeed. This establishes
which writes are required; the historical bubblewrap result alone does not prove
why the original host environment denied them. SQLite WAL creation also requires
a writable containing directory; binding individual database filenames fails for
fresh WAL initialization.

Completed budget-ship sessions had another defect: typed command folds dropped
updated risk, lens, and clean-minimum material. The compatibility projection
looked valid while the next command rejected its contract. The fix preserves the
facts emitted by the authoritative command rather than recalculating a hash for
arbitrary invalid state.

## Decision

Keep independent review requirements and attestations. Recover persistence by
replaying one exact operation with the same version-checked binary through the
supported approval mechanism and narrow writable authority paths. Repository
source stays read-only and network access is denied in the recovery process.
Git remains authoritative; rebuild disposable projections rather than copying
scratch databases over concurrent state.

Use explicit, validated successor operations with optimistic concurrency and
durable idempotent receipts. Preserve immutable baselines, snapshot bindings,
append-only evidence, and interrupted-operation recovery. A completed session
reopens through a recorded intent, clears stale clean credit, and obtains fresh
independent delta assessment and review coverage.

Treat each 75-minute boundary as a progress assessment. Authorized work may
continue through a recorded continuation without asking a human when no human
decision is needed. Preserve original start time and clean requirements. Resolve
legacy escalation holds with concrete recovery evidence. Timer continuation is
never a waiver of review, security, signing, verification, or delivery gates.

Persist independently rejected findings with stable identity and the evidence
that justified rejection. Reviewers can challenge a resolution by explaining
why it no longer holds and supplying new contradictory evidence, a relevant
change, or a demonstrated verification gap. Identity and evidence determine
reuse; wording similarity does not. Evidence dependencies determine applicability,
so unrelated documentation changes do not discard source-bound resolutions.

Report completed review rounds separately from iteration and reset bookkeeping.
Separate raw allegations from adjudicated outcomes and provide inspectable
references for every reported count. An all-clean round contributes required
independent scrutiny; its yield does not change policy automatically.

Repair credit requires a prior independently confirmed defect, a changed
remediation path in the immutable reviewed snapshots, and fresh independent
evidence rejecting the original failure. An unrelated source change or a
caller-provided hash cannot establish a repair. A verifier that still confirms
the failure preserves the defect rather than earning repair credit.

## Consequences

Recovery requires the supported Linux bubblewrap runtime and approval for the
exact scoped operation when ordinary permissions deny persistence. Failure to
unlock signing remains a signing dependency, not a reason to stop independent
work. Legacy records without sufficient evidence are reported as unavailable;
new reporting must not invent historical adjudication or review rounds.

The existing projection retains 64 round-history rows. Reporting explicitly
marks a truncated window and leaves full-history totals and finding novelty
unavailable; observed counts retain inspectable evidence. Latest resolution
outcomes survive this pruning with their original proofs and reopening
tombstones. Git event history remains append-only. The existing state-size
limit still fails closed when retained evidence exceeds its bound.
