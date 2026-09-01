# ADR-0014: Support Codex exclusively

## Status

Accepted

## Date

2026-08-31

## Context

The marketplace maintained parallel packaging, setup, routing, validation, and
documentation for Codex and Claude Code even though its canonical behavior
evaluation and active development workflow already ran through Codex. The
duplicated surface increased maintenance cost and made the supported product
boundary unclear.

## Decision

Support Codex exclusively beginning with Development System 6.0.0. The Codex
marketplace and `.codex-plugin` manifests are the only distribution contract.
Repository setup writes Codex project configuration, model routing uses Codex
model identifiers, and behavior evaluation uses the native Codex SDK provider.

Remove Claude-specific manifests, hooks, agents, runtime branches, dependencies,
tests, eval fixtures, and current documentation. Do not provide compatibility
shims for the removed interfaces. Historical ADRs and archived implementation
plans remain accurate records of earlier decisions; this ADR supersedes their
support assumptions wherever they conflict.

## Consequences

Development System 5.5.x is the final dual-harness release. Existing Claude
Code consumers must remain on that line or migrate to Codex. New features,
validation, and behavior evidence target Codex only, reducing duplicated
maintenance and making the repository's support boundary explicit.

## Alternatives considered

Continuing deterministic Claude packaging without Claude behavior evals would
retain most of the compatibility burden without equivalent evidence. Keeping a
generic multi-harness abstraction would preserve unused branches and obscure
the actual product commitment. Both were rejected.

## Revisit when

Revisit only through a new architectural decision backed by a concrete product
need, an owned compatibility surface, and harness-specific behavior evidence.
