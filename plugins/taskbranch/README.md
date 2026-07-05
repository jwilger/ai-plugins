# taskbranch

Git-backed task boards for coding agents.

`taskbranch` stores task state in a dedicated orphan `tasks` branch and exposes
the active worktree's board through `.tasks`. The canonical branch path is
`<worktree-name>/.tasks`; linked worktrees use symlinks so every checkout can
see its own board without mixing task files into the source branch.

The plugin ships the Rust source for auditability and development, plus a
`bin/taskbranch` launcher that prefers bundled release binaries from `dist/`
when present.

## CLI

The public binary is named `taskbranch`.

```shell
taskbranch init
taskbranch create "Write taskbranch docs"
taskbranch list
taskbranch metadata todo/write-taskbranch-docs.md
```

The CLI also includes `sync`, `show`, `metadata`, `next`, `transition`,
`prioritize`, `link`, `unlink`, `subtask add|check|uncheck`, `validate --fix`,
`close-from-trailers`, `dashboard serve`, `mcp stdio`, and
`scaffold repo --dry-run|--apply`.

## Storage model

- `tasks` is an orphan Git branch that owns task-board state.
- The active worktree points `.tasks` at `.git/taskbranch/<worktree>/.tasks`.
- Task status is represented by directories such as `todo/`.
- Task identity is the filename, and `order.md` records board order.
- `taskbranch metadata <ref>` reports the task's latest `tasks` branch commit
  time when that task has been synced.
- Writes must sync through the task branch before they are considered durable.

## Stdio MCP

The supported MCP transport is stdio:

```shell
taskbranch mcp stdio
```

## Harnesses

Taskbranch targets both Claude Code and Codex. The same plugin name, binary
name, stdio MCP server name, and documentation name are used everywhere:
`taskbranch`.
