# Tiber

Tiber is a Git-backed task board for coding agents. It keeps task state in a
dedicated orphan `tasks` branch, exposes the current worktree's board at
`.tasks`, and gives agents a deterministic CLI plus stdio MCP tools for creating,
ordering, validating, and closing repository-local work.

The goal is simple: multiple agents and worktrees can coordinate without turning
task files into untracked side chatter or hand-edited markdown drift.

## Screenshots

These screenshots were captured from this repository after initializing Tiber and
creating the top-level tasks for the rename/merge work.

![Tiber dashboard showing current repository tasks](docs/screenshots/dashboard-board.png)

![Tiber task detail page for the documentation task](docs/screenshots/task-detail.png)

![Tiber docs browser listing repository markdown docs](docs/screenshots/docs-browser.png)

## Quick Start

```shell
tiber init
tiber create "Document release checklist"
tiber list
tiber show todo/document-release-checklist.md
tiber validate --fix
tiber sync
```

`tiber init` is explicit. Installing the plugin or starting an agent session does
not mutate the repository.

## What Tiber Stores

- `tasks` is an orphan Git branch that owns task-board state.
- Each worktree has its own canonical board under
  `<worktree-name>/.tasks` on that branch.
- The source checkout gets a `.tasks` symlink into Git's common directory so the
  active worktree can read and write its board naturally.
- Task status is represented by directories such as `todo/`, `doing/`, and
  `done/`.
- `order.md` records board order.
- Task identity is the markdown filename, not the display title.

This keeps task state versioned, syncable, and separate from the source branch
while still making it inspectable from the worktree.

## CLI Commands

Common reads:

```shell
tiber list
tiber next
tiber show <task-ref>
tiber metadata <task-ref>
```

Common writes:

```shell
tiber create "Task title"
tiber transition <task-ref> <status>
tiber prioritize <task-ref> --before <task-ref>
tiber link <task-ref> blocks <task-ref>
tiber unlink <task-ref> blocks <task-ref>
tiber subtask add <task-ref> "Subtask title"
tiber subtask check <task-ref> 1
tiber subtask uncheck <task-ref> 1
```

Validation and integration:

```shell
tiber validate --fix
tiber close-from-trailers
tiber scaffold repo --dry-run
tiber scaffold repo --apply
```

`validate --fix` only performs safe mechanical repairs: misplaced claims,
missing reciprocal links, and `order.md` reconciliation. Dangling references and
dependency cycles are reported for deliberate resolution.

## Sync Model

Tiber writes local board changes, commits them to the `tasks` branch, and pushes
that branch when an `origin` remote exists.

Write sync conflicts are hard failures. Do not force-push or choose a side
blindly. Preserve both sides, resolve the conflict deliberately, then rerun:

```shell
tiber sync
```

Read commands perform a soft remote sync attempt, but remote fetches are
non-interactive and time-bounded so a slow or unreachable remote does not hang
normal reads.

## Stdio MCP

Tiber exposes the same task operations over stdio MCP:

```shell
tiber mcp stdio
```

The plugin manifest registers this server as:

```json
{
  "mcpServers": {
    "tiber": {
      "command": "./bin/tiber",
      "args": ["mcp", "stdio"]
    }
  }
}
```

Tool names use the `tiber.*` namespace, for example `tiber.create`,
`tiber.list`, `tiber.transition`, and `tiber.validate_fix`.

## Dashboard

The dashboard is a read-only browser view:

```shell
tiber dashboard serve
```

Open `http://127.0.0.1:7417/` to inspect the board, task files, and repository
docs. The dashboard intentionally does not expose write routes, `/mcp`, or SSE
event routes. Task changes go through the CLI or stdio MCP tools.

## Scaffold Workflow

Repository integration is dry-run first:

```shell
tiber scaffold repo --dry-run
```

The preview covers:

- `.gitignore` entries for the local `.tasks` symlink
- a post-commit hook for trailer-based closing
- a GitHub workflow snippet for `tiber close-from-trailers`
- an optional `just show-tasks` recipe when a `justfile` exists

Apply only after explicit approval of the preview:

```shell
tiber scaffold repo --apply
```

## Release Layout

The plugin ships:

- Rust source under `rust/`
- a `bin/tiber` launcher
- prebuilt binaries under `dist/<target>/tiber`
- release metadata in `release-binaries.json`

The launcher prefers a matching bundled binary and falls back to
`cargo run --manifest-path rust/Cargo.toml --bin tiber` for development.

## Harness Support

Tiber targets both Claude Code and Codex. The same plugin name, skill name, CLI
binary, MCP server name, and documentation name are used everywhere: `tiber`.
