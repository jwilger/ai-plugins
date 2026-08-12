# ADR-0005: Use codex app-server as the sole inference transport

## Status

Accepted after the Phase 1 effective-authority spike

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

The first Phase 1 conclusion incorrectly equated protocol availability with
effective authority. `commandExecution` and `fileChange` thread items describe
observable work; their presence does not prove that an operation can cross the
configured permissions boundary.

Codex CLI 0.147.0 exposes the controls Tiber needs. A named permission profile
can enforce a read-only filesystem and disabled command network, while
`approvalPolicy = "never"` returns denied effects to the model instead of
allowing an escalation. Separate controls disable hosted web search, shell,
permission requests, apps, browser, Computer Use, image generation, subagents,
and related host surfaces. An isolated Codex home contains no user MCP servers,
plugins, hooks, agents, or configuration. App-server delivers a declared
dynamic tool through `item/tool/call`; the client owns its response and effect
execution. The named profile deliberately allows read-only, non-shell
repository observation as untrusted inference context while reserving every
effect and durable decision for Tiber. Tiber resolves the exact app-server
executable and generates a read-only grant for that file because Codex uses it
as the Linux sandbox helper; no broader user-home grant is needed.

The live x86_64 Linux probe selected the `tiber-inference` profile, observed a
Tiber sentinel dynamic-tool call as inert client data, and deliberately
executed no dynamic-tool effect. A positive-control `command/exec` using the
probe's known Node executable first exited successfully without mutation; the
same executable's write attempt under that profile then returned a nonzero
result and created no sentinel file. The effective sandbox reported `readOnly` with
command network disabled, and hosted web search was disabled independently.
Protocol structure remains pinned and fails closed on drift.

## Consequences

Tiber can continue with app-server as its sole inference transport. Built-in
operation names need not disappear from the protocol or model vocabulary; the
required invariant is that no operation outside Tiber policy can produce an
effect. Tiber must retain the isolated-home configuration, decline every
approval and permission-escalation request, execute dynamic tools only through
its typed dispatcher, and rerun the effective-authority probe for every
supported Codex protocol/version range.

## Alternatives considered

Direct Responses API and Anthropic providers were rejected because they
duplicate auth/transport responsibility and expand the v1 matrix.

## Revisit when

Codex changes permission-profile enforcement, app-server approval or dynamic
tool semantics, relevant feature controls, or the pinned protocol surface.
