# ADR-0005: Use codex app-server as the sole inference transport

## Status

Accepted

## Date

2026-08-10

## Context

Tiber needs Codex inference and subscription login without handling credentials
or giving the inference runtime workflow authority.

## Decision

Use `codex app-server` exclusively. Subscription login is default and
app-server-mediated API-key login is the explicit fallback. Run it with an
isolated Tiber Codex home, pin a compatible protocol range, and parse tool calls
as inert data. Tiber never reads or logs credentials.

## Consequences

There is one auth and inference integration. The Phase 1 spike blocks further
work unless no undeclared app-server tool is advertised or executable.

## Alternatives considered

Direct Responses API and Anthropic providers were rejected because they
duplicate auth/transport responsibility and expand the v1 matrix.

## Revisit when

App-server cannot meet tool isolation, loses a supported protocol, or cannot
provide required subscription access.
