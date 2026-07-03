---
name: side-quest
description: Launch, monitor, and inspect background side-quests through the sidequest MCP server. Use when the user asks Codex to delegate a change to a background worker, run work in an isolated git worktree, check running side-quests, inspect side-quest logs, or report whether a side-quest has delivered, failed, or is awaiting input.
---

# Side Quest

Use the `sidequest` MCP server. Do not implement side-quest orchestration by
hand when the MCP tools are available.

## Launch

When the user asks to start, launch, run, delegate, or background a side-quest:

1. Pass the user's requested work as the `goal` argument to the `launch` tool on
   the `sidequest` MCP server.
2. Preserve the goal text as the user phrased it. Do not rewrite it into a shell
   command.
3. Report the returned branch and worktree path.
4. Tell the user the side-quest continues independently after the current
   session unless the tool result says otherwise.

## Monitor

When the user asks what side-quests are running or asks for status, call the
`list` tool. Use the returned `state` exactly:

- `running`: still working.
- `awaiting-input`: blocked on a decision or response.
- `delivered`: changes were delivered according to `sidequest.toml`.
- `done`: completed.
- `done-no-changes`: ran successfully but produced no changes to deliver.
- `failed`: failed; include the returned `detail`.

## Logs

When the user asks what a side-quest is doing, asks why it is blocked, or asks
for recent output, call the `logs` tool with the side-quest branch. Prefer the
branch from the latest `launch` or `list` result. If multiple branches match,
ask which branch to inspect.

Summarize logs instead of pasting long raw output. Include exact error messages
or requested user decisions when they are the reason action is needed.

## Missing Tools

If the `sidequest` MCP server or tools are unavailable, do not pretend a
side-quest launched. Report that the MCP server is unavailable and point to the
plugin README requirement that `sidequest-mcp` must be installed on `PATH`.
