# ADR-0005: Use codex app-server as the sole inference transport

## Status

Rejected by the Phase 1 compatibility spike

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

## Spike outcome

Codex CLI 0.147.0 does not satisfy the authority contract. Its generated
app-server protocol includes `commandExecution` and `fileChange` thread items,
while `thread/start` provides `dynamicTools` only as an additive experimental
facility and no documented built-in-tool disable or allowlist contract. The
official protocol documentation likewise distinguishes dynamic client tools
from Codex-owned command and file-change work.

The executable probe under `tiber/` fails closed with
`app_server_tool_isolation_unverified`. This is not repaired by selecting a
read-only sandbox or `approvalPolicy = "never"`: those settings constrain
effects but do not prove that the model cannot receive or app-server cannot own
undeclared built-in operations.

## Consequences

The original decision cannot proceed without verified evidence for Tiber's
central authority boundary. In accordance with the product gate, roadmap
construction stops after the spike. A replacement inference decision requires an app-server
protocol that can expose only Tiber-declared inert tools, or a different
transport explicitly approved in a new ADR.

## Alternatives considered

Direct Responses API and Anthropic providers were rejected because they
duplicate auth/transport responsibility and expand the v1 matrix.

## Revisit when

App-server adds a documented, testable built-in-tool disable/allowlist contract
that covers both model advertisement and execution.
