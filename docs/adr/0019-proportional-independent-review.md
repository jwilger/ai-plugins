# 0019: Proportional independent review coverage

Status: Accepted

## Context

Repeated full clean rounds re-review unchanged areas and confuse generator-source changes with visual changes. Correctness depends on independent applicable coverage, trustworthy evidence, and causal repair verification rather than a universal count.

## Decision

Development System protocol v3 defaults to one complete independently risk-selected coverage set. Every applicable or uncertain dimension must be selected and every changed path covered. Consequential high or exceptional risks require explicit uncertainty, consequence, and sample-count rationale for additional samples of affected responsibilities. Existing exceptional trigger definitions remain intact.

Receipts retain original reviewer, source snapshot, shared verification identity, dependency identities, and lifecycle attestations. Independently assessed repairs invalidate affected behavior and dependencies; whole-scope impact or unisolatable provenance requires complete renewal. Unchanged source, behavior, scope, and freshness permit evidence reuse with current applicability proof. Deleted dependencies require host-observed absence. Visual reuse requires current render continuity; equivalent outputs can preserve visual judgments despite changed generator inputs, while code/model judgments retain their own source dependency rules.

Separate model/document and rendered-artifact responsibilities from production-code responsibilities. Security, boundary checks, regressions, exact-revision readiness, actual CI/CodeRabbit, signing, and failure recovery remain required. Evidence reuse never invents new reviewer runs or concrete model identity.

V2 sessions keep their original three-round contract. Explicit audited migration closes or accounts for prior assignments, preserves source, findings, holds, and history, and requires genuine independent v3 assessment; absent old provenance is never inferred. The standalone unbound Tiber review scheduler is unchanged. Optional task-board minima one and two become valid without weakening committed receipts, independence, fingerprints, or trailer enforcement.

## Consequences

Coordinator state and model-facing instructions share explicit coverage and invalidation semantics. Behavioral tests must reject under-review and false provenance as well as unnecessary full resets. Changed defaults and serialized coverage require a major release. No new general orchestration layer is introduced.
