# Tiber

Tiber is a Git-backed task board for coding agents. It keeps task state in a
dedicated orphan `tasks` branch and gives agents a deterministic CLI plus stdio
MCP tools for creating, ordering, validating, and closing repository-local work.

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
tiber show document-release-checklist
tiber validate --fix
tiber sync
```

`tiber init` is explicit. Installing the plugin or starting an agent session does
not mutate the repository.

When you start working on an existing task, move it out of the backlog first:

```shell
tiber transition <task-ref> in-progress
```

Backlog tasks are unclaimed work, not informal reservations.

## What Tiber Stores

- `tasks` is an orphan Git branch that owns task-board state.
- The branch stores a shared tree with `backlog/`, `in-progress/`,
  `done/`, and `abandoned/` status directories.
- Tiber reads and writes that branch through Git object/tree/ref operations. It
  does not keep the `tasks` branch checked out locally and does not create a
  persistent `.tasks` working copy.
- Task files are named `<YYYYMMDD-xxxx>-<nickname>.md` and contain YAML
  frontmatter plus standard Markdown sections.
- `order.md` records one bare task stem per line for open work only.
- CLI and MCP commands accept a task id, nickname, or full stem. Users do not
  need to pass `.tasks` paths, status directories, or Markdown section names.

This keeps task state versioned, syncable, and separate from the source branch.
Inspect it through `tiber show`, `tiber list`, the read-only dashboard, or normal
Git commands such as `git show tasks:order.md`.

## New Task Skill

The plugin includes the manually invokable `tiber:new-task` skill for quick
backlog capture from an agent session:

```text
tiber:new-task Document release checklist
```

The skill creates the task through structured Tiber MCP tools, records any
obvious summary or acceptance details from the prompt, runs the structured Tiber
MCP validation tool, and leaves the task in `backlog` unless the user explicitly
asks to start work immediately.

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
tiber subtask add <task-ref> "Dependent subtask" --after s1,s2
tiber subtask check <task-ref> s1
tiber subtask uncheck <task-ref> s1
tiber update <task-ref> --summary "New summary" --tags infra,docs
tiber update <task-ref> --pr-mr-url https://github.com/org/repo/pull/42 --pr-mr-status checks-pending
tiber acceptance add <task-ref> "Observable condition"
tiber acceptance check <task-ref> 1
tiber note add <task-ref> "Progress note"
```

Validation and integration:

```shell
tiber validate --fix
tiber close-from-trailers
tiber install-bin --target-dir ~/.local/bin --dry-run
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

Read commands sync before returning task data. If Tiber can merge remote task
state automatically, the read continues with the merged board. If the sync cannot
be resolved automatically, the read fails instead of returning stale or locally
divergent task data.

## Stdio MCP

Tiber exposes the same task operations over stdio MCP:

```shell
tiber mcp stdio
```

The plugin manifest registers this server through an absolute `/bin/sh` launcher
that resolves the installed `bin/tiber` from Claude's `${CLAUDE_PLUGIN_ROOT}`
when that variable is set, or from the exact `tiber/0.5.0` Codex plugin cache
when running under Codex. If `${CLAUDE_PLUGIN_ROOT}` is set but does not contain
an executable `bin/tiber`, startup fails with
`tiber.mcp_claude_plugin_root_invalid` rather than falling back to another
cache. If `${CODEX_HOME}` is set but the exact Codex cache entry is missing,
startup fails with `tiber.mcp_codex_cache_missing`; only sessions without an
explicit `${CODEX_HOME}` fall back to `$HOME/.codex`.

It intentionally does not execute repo-relative launchers such as `./bin/tiber`
or `./plugins/tiber/bin/tiber`, so the same MCP configuration is safe to load
from any checkout. Reinstall or upgrade the plugin from marketplace version
`0.5.0` or newer if Codex reports `No such file or directory` or one of the
`tiber.mcp_*` startup sentinel errors while starting the `tiber` MCP server.

Tool names use the `tiber.*` namespace, for example `tiber.create`,
`tiber.list`, `tiber.transition`, `tiber.update`, `tiber.acceptance.add`,
`tiber.note.add`, `tiber.install_bin`, and `tiber.validate_fix`.

## Dashboard

The dashboard is a read-only browser view:

```shell
tiber dashboard serve
```

Open `http://127.0.0.1:7417/` to inspect the board, task files, and repository
docs. The dashboard exposes a read-only `/events` SSE stream for live refreshes,
but intentionally does not expose write routes or `/mcp`. Task changes go
through the CLI or stdio MCP tools.

## Scaffold Workflow

Repository integration is dry-run first:

```shell
tiber scaffold repo --dry-run
```

The preview covers:

- `.gitignore` entries preventing accidental source-branch `.tasks` checkouts
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
- checksum provenance in `release-binaries.sha256`

The launcher prefers a matching bundled binary and falls back to
`cargo run --manifest-path rust/Cargo.toml --bin tiber` for development.
Generate the release metadata and checksums with
`scripts/build-tiber-release-all.sh`.

## Harness Support

Tiber targets both Claude Code and Codex. The same plugin name, skill name, CLI
binary, MCP server name, and documentation name are used everywhere: `tiber`.
