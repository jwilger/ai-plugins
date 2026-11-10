# 24. Explicit policy release of obsolete CI recovery holds

Status: Accepted

## Context

Repository policy can retire a historical incident hold while its failure and
cause remain unresolved. Successful replacement proof would misrepresent this
change. ADR 0002 allowed only successful recovery to release the hold.

## Decision

Supersede the success-only hold-release clause of ADR 0002. Add an explicit
owner-fenced `release-by-policy` command and matching MCP operation. Require an
active lease, current epoch, policy reference and rationale. Persist a typed
policy-release fact with incident, epoch, actor and server time. Project the
separate terminal `policy-released` phase and audit metadata without adding a
success proof or changing original evidence. Preserve immutable history and
normal Git synchronization, concurrency and replay guarantees.

## Consequences

Distinct subsequent failures may be claimed. Ordinary `resolve` still requires
exact successful replacement evidence. No automatic workflow exemptions or
historical-record reinterpretation occur. Policy authority remains an explicit
operator responsibility; a reference is audit evidence, not executable policy.

## Alternatives and revisit conditions

Reject fabricated successful replacement evidence, silent workflow exemptions,
and manual event-store edits because they erase the distinction between policy
and recovery. Reject a separate administrative override because current fenced
ownership already serializes repository decisions. Revisit if repositories need
machine-verified policy documents or a separately authorized administrative role;
neither is implied by a caller-supplied reference.
