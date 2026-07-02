---
description: Launch a backgrounded side-quest that implements a change in its own git worktree.
argument-hint: <goal describing the change to make>
---

Launch a side-quest for this goal:

$ARGUMENTS

Call the `launch` tool on the `sidequest` MCP server, passing the goal text above
as the `goal` argument. The side-quest runs in its own isolated git worktree and
delivers its work according to the project's `sidequest.toml`. When the tool
returns, report the side-quest's branch and worktree path from the result.

To check on it later, use the `list` tool (its `state` field: `running`,
`awaiting-input`, `delivered`, `done`, `done-no-changes`, or `failed` with a
`detail`) and the `logs` tool (its captured session output so far, given its
branch).
