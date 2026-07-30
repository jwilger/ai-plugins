# Pi subagent observability research

Date: 2026-07-30

## Question and method

Development-system already starts fresh Pi subprocesses for final-review coordinator assignments. The problem was not delegation itself: a parent tool row could remain opaque for minutes. This research compared current public Pi subagent extensions for patterns that expose useful progress without copying child transcripts, credentials, or broad trust assumptions.

Repository metadata, release metadata, npm metadata, READMEs, and selected runtime sources were inspected through the public GitHub and npm APIs. Adoption counts are point-in-time signals, not quality guarantees.

## Comparison

| Implementation                                                                        | Visibility and control                                                                                                                                                                                                      | Maintenance/adoption signal on 2026-07-30                                                                                                                                                 | Security and trust observations                                                                                                                                                                                                                                                                                                                                                                                                                                                                      |
| ------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| [`nicobailon/pi-subagents`](https://github.com/nicobailon/pi-subagents)               | Foreground progress, background status, a persistent fleet view, transcript inspector, structured lifecycle artifacts, tool/turn counts, stop/interrupt/steer/resume, nested-run projection, and bounded protocol handling. | 2,789 stars, 432 forks, 2 open issues; pushed 2026-07-29; MIT; npm [`pi-subagents@0.37.2`](https://www.npmjs.com/package/pi-subagents) and GitHub release `v0.37.2` published 2026-07-28. | Strongest mature observability reference. It explicitly separates tool visibility from runtime permission policy, caps child protocol lines and stderr, constrains nested routes with capability tokens, and documents that tool restrictions are not an OS sandbox. Its large orchestration surface, extension inheritance, optional integrations, persisted transcripts, and install/runtime code create substantially more supply-chain and data-retention surface than development-system needs. |
| [`HazAT/pi-interactive-subagents`](https://github.com/HazAT/pi-interactive-subagents) | Non-blocking children in tmux/cmux panes, a live status widget, child activity snapshots, stalled/recovered classification, expandable completion cards, turn interruption, resume, and a child-to-parent help channel.     | 607 stars, 110 forks, 23 open issues; pushed and released as MIT `v3.7.2` on 2026-05-12.                                                                                                  | Excellent direct human visibility and an explicit distinction between active, waiting, and stalled work. Multiplexer panes, command delivery, session files, and user takeover are intentionally interactive but add terminal/process integration and transcript exposure that do not fit a narrow headless review child. Tool denial is extension-level policy, not process sandboxing.                                                                                                             |
| [`mjakl/pi-subagent`](https://github.com/mjakl/pi-subagent)                           | Streams progress from Pi RPC events and renders collapsed/expanded TUI details with tool calls, usage, output, and session metadata. Uses settlement events and supports parallel calls and cycle/depth guards.             | 73 stars, 15 forks, 0 open issues; pushed 2026-07-10; MIT; no GitHub release found.                                                                                                       | A smaller, readable subprocess design. It bounds model-facing output to 50 KiB/2,000 lines and process capture to a rolling 5 MiB, with mode-0600 temporary summaries. Expanded views intentionally retain more child messages and tool details than development-system should expose by default. Inherited extensions and ordinary child tools remain trusted execution surface.                                                                                                                    |
| [`vnedyalk0v/pi-subagent-kernel`](https://github.com/vnedyalk0v/pi-subagent-kernel)   | Specifies stable run IDs, structured JSONL events, status/inspect/cancel commands, backend identity, activity, usage, file-change signals, artifacts, and text-first fallback.                                              | 2 stars, 0 forks, 3 open issues; pushed 2026-07-11; no recognized SPDX license or GitHub release.                                                                                         | Its event-design rules are strong: no secrets, no chain-of-thought, no implicit full file contents, bounded artifacts, and no telemetry without opt-in. Low adoption, no recognized license, and a broader backend/registry product scope make it a useful design comparison but not a suitable implementation dependency or code source.                                                                                                                                                            |

The bundled upstream [Pi subagent example](https://github.com/earendil-works/pi-mono/tree/main/packages/coding-agent/examples/extensions/subagent) is not third-party, but it confirms the supported mechanism: run a child in JSON mode, consume Pi lifecycle/tool events, and render bounded progress through extension tool updates.

## Selected reference

`nicobailon/pi-subagents` is the reference implementation for observability semantics because it has the clearest combination of current maintenance, adoption, lifecycle artifacts, fleet/status UX, bounded protocol handling, and explicit security caveats.

Development-system does **not** depend on or copy that package. Its general orchestration, background fleet, transcript browser, nested fanout, persistent artifacts, optional permission integration, and recovery APIs exceed this ticket. The first-party implementation adopts only these ideas:

1. consume Pi's supported JSON event stream rather than scraping human terminal output;
2. show current lifecycle state plus a known built-in tool name or generic first-party/extension category, never arbitrary tool names, arguments, or results;
3. retain heartbeat-updated elapsed time, aggregate turn/tool/error/active counts, parallel-tool state, and only the latest 20 sanitized events;
4. bound a protocol line to 4 MiB, the complete event stream to 64 MiB, stderr to 128 KiB without retaining it, the final structured result to 50 KiB, and coalesce parent updates to at most four per second plus one final snapshot;
5. keep cancellation, timeout, provider failure, protocol/output-limit failure, and malformed-result outcomes explicit, escalating resistant child process groups and awaiting close; and
6. preserve fresh-process isolation while returning no child transcript or stderr content.

## Supply-chain and execution risks

Pi's extension documentation states that extensions execute with the user's full permissions. Installing any reviewed package therefore trusts its published artifact, maintainer account, dependency graph, install hooks, and future updates. Star counts and MIT licensing do not reduce that authority.

A child Pi process also runs models and tools with the authority granted by its effective Pi configuration. Tool allowlists are useful least-authority controls but are not an operating-system sandbox. Auth must remain available to the selected provider, so observable events must never serialize environment variables, credential stores, request headers, stderr, raw prompts, or unrestricted transcripts.

The selected first-party design adds no runtime dependency and no network service. It continues to trust the repository owner, installed Pi binary, local environment, and configured provider, consistent with this repository's proportional threat model. It treats accidental disclosure, runaway output, hangs, malformed protocol data, and interrupted process trees as in-scope failures.

## Sources

- Pi extension security and custom-tool updates: <https://github.com/earendil-works/pi-mono/blob/main/packages/coding-agent/docs/extensions.md>
- Pi SDK event stream: <https://github.com/earendil-works/pi-mono/blob/main/packages/coding-agent/docs/sdk.md>
- Pi bundled subagent example: <https://github.com/earendil-works/pi-mono/tree/main/packages/coding-agent/examples/extensions/subagent>
- pi-subagents repository and README: <https://github.com/nicobailon/pi-subagents>
- pi-subagents nested event sanitization: <https://github.com/nicobailon/pi-subagents/blob/main/src/runs/shared/nested-events.ts>
- pi-interactive-subagents repository and README: <https://github.com/HazAT/pi-interactive-subagents>
- mjakl/pi-subagent repository: <https://github.com/mjakl/pi-subagent>
- mjakl Pi event capture: <https://github.com/mjakl/pi-subagent/blob/main/runner-events.js>
- pi-subagent-kernel observability design: <https://github.com/vnedyalk0v/pi-subagent-kernel/blob/main/docs/08-observability-ux.md>
- npm package metadata: <https://www.npmjs.com/package/pi-subagents>
