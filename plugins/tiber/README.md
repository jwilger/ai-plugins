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

For Codex sandboxed sessions, preview the narrow host-access setup before
granting broad permissions:

```shell
tiber codex-sandbox --dry-run
```

The preview treats raw Git prefix approvals as case-by-case, including
`hash-object`, `mktree`, `commit-tree -S`, `update-ref`, fetch, and push.
Persist approval only when the harness can scope it to the exact Tiber-internal
operation rather than a reusable raw `git` prefix. Prefer the narrowest approval
that lets the same structured Tiber MCP operation be retried. Do not run the
whole Tiber MCP server outside the sandbox unless the narrow Git permissions are
insufficient.

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

It relies only on structured Tiber MCP tools for creation, validation, and
backlog handling. It does not fall back to the Tiber CLI, direct file edits, or
shell commands.

## CLI Commands

Run `tiber --help` for the complete command list and
`tiber <command> --help` for parser-generated command usage. Generated help is
the authoritative CLI grammar.

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
tiber codex-sandbox --dry-run
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

`close-from-trailers` synchronizes the authoritative board before resolving
`Closes:` lines from the current `HEAD` commit. It prints `closed <task-id>` for
each published transition and exits nonzero when a requested task is missing,
invalid, conflicted, or cannot be pushed. A successful run therefore guarantees
that every task it names is no longer open.

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
when that variable is set, or from the exact `tiber/0.10.4` Codex plugin cache
when running under Codex. If `${CLAUDE_PLUGIN_ROOT}` is set but does not contain
an executable `bin/tiber`, startup fails with
`tiber.mcp_claude_plugin_root_invalid` rather than falling back to another
cache. If `${CODEX_HOME}` is set but the exact Codex cache entry is missing,
startup fails with `tiber.mcp_codex_cache_missing`; only sessions without an
explicit `${CODEX_HOME}` fall back to `$HOME/.codex`.

The Codex MCP registration forwards `SSH_AUTH_SOCK` so Git SSH signing can use
the user's existing agent, including 1Password SSH agent setups. If an older
installed plugin still reports `Couldn't get agent socket?` during
`git commit-tree -S`, reinstall Tiber or replace the plugin-provided server with
an equivalent top-level `[mcp_servers.tiber]` registration that preserves the
absolute installed launcher and includes `env_vars = ["SSH_AUTH_SOCK"]`. Do not
forward `SSH_AUTH_SOCK` to `command = "tiber"`, repo-relative launchers, or any
project-controlled executable. Codex plugin MCP policy overlays under
`[plugins."tiber@ai-plugins".mcp_servers.tiber]` cannot change transport
environment variables; they only control enablement and tool policy.

It intentionally does not execute repo-relative launchers such as `./bin/tiber`
or `./plugins/tiber/bin/tiber`, so the same MCP configuration is safe to load
from any checkout. Reinstall or upgrade the plugin from marketplace version
`0.6.1` or newer if Codex reports `No such file or directory` or one of the
`tiber.mcp_*` startup sentinel errors while starting the `tiber` MCP server.

Tool names use the `tiber.*` namespace, for example `tiber.create`,
`tiber.list`, `tiber.transition`, `tiber.update`, `tiber.acceptance.add`,
`tiber.note.add`, `tiber.codex_sandbox_setup`, `tiber.install_bin`, and
`tiber.validate_fix`.

The `initialize` response also points Codex agents at
`tiber.codex_sandbox_setup` and `tasks://codex-sandbox` so sandbox setup can be
discovered through MCP before retrying a failed write.

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

- an additive `.gitignore` update that preserves existing entries and adds
  `.tasks` at most once
- a post-commit hook for trailer-based closing
- a GitHub workflow snippet for `tiber close-from-trailers`
- an optional `just show-tasks` recipe when a `justfile` exists
- explicit `would write`, `already configured`, and `conflict` classifications;
  equivalent existing hooks and workflows suppress duplicate automation

Apply only after explicit approval of the preview:

```shell
tiber scaffold repo --apply
```

Apply refuses to overwrite a conflicting generated hook or workflow path. After
reviewing every reported conflict, explicitly choose replacement with:

```shell
tiber scaffold repo --apply --replace-conflicts
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
