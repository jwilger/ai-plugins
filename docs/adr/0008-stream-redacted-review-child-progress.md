# ADR 0008: Stream redacted review-child progress

## Status

Accepted

## Context

`development_system_run_review_assignment` runs a fresh, isolated Pi subprocess and waits for one structured result. The parent previously received only a startup update and could show an unchanged tool row for up to the ten-minute timeout. Users could not distinguish active tool work from a stuck provider or process.

Raw child JSON events contain model messages, prompts, tool arguments, tool results, paths, and provider diagnostics. Forwarding or retaining that stream would expose private repository content and create unbounded parent context.

## Decision

Run review children with Pi's supported `--mode json` event stream and project it into a small first-party progress contract.

The projection exposes:

- lifecycle state: `starting`, `running`, `tool-running`, `tool-completed`, `responding`, or `settled`;
- heartbeat-updated elapsed time;
- aggregate turn, tool-call, tool-error, and active-tool counts;
- a known built-in tool name or a generic first-party/extension category; and
- at most 20 recent synthetic lifecycle events.

It does not expose the assignment, model text, thinking, arbitrary tool names, tool arguments, paths, tool results, stderr, environment, authentication state, or raw transcript. The shell retains only one incomplete JSONL line, caps that line at 4 MiB, caps the complete child event stream at 64 MiB and stderr at 128 KiB without retaining stderr content, retains only the latest assistant text needed for the structured coordinator result, and caps that result at 50 KiB. Event-driven parent updates are coalesced to at most four per second, with the latest state forced once at the child close boundary.

The parent abort signal and the existing ten-minute timeout request process-group termination with `SIGTERM`, escalate resistant children to `SIGKILL`, and wait for the child close boundary before reporting failure. Successful and provider-failed root exits also detect and close retained Unix process-group descendants before returning or attesting closure. Cancellation, timeout, output-limit, provider, spawn, and malformed-result failures remain structured unresolved outcomes and now include the same bounded progress snapshot.

## Consequences

The parent tool row now changes as the child reasons and uses tools, while headless callers receive the same structured progress in tool-update details. The final result remains backward-compatible and gains a `progress` field.

This is foreground review-child observability, not a general background task broker, transcript viewer, or OS sandbox. Child tools and provider authentication retain their existing configured authority. Adding background execution, transcript inspection, steering, durable run registries, or broader subagent APIs requires a separate decision and threat model.

The external comparison and source rationale are recorded in [`docs/research/pi-subagent-observability.md`](../research/pi-subagent-observability.md).
