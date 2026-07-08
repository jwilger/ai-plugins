---
name: tiber
description: Use when the user wants repository task tracking, shared agent task state, cross-worktree coordination, tiber setup/install/scaffold guidance, or task create/list/show/prioritize/validate/close workflows. Plugin install and session start are non-mutating; setup integration starts with dry-run previews.
---

# Tiber

Use Tiber for repository-local task boards backed by the Git `tasks` branch and
a shared `backlog/`, `in-progress/`, `done/`, and `abandoned/` task tree. Tiber
uses Git object/tree/ref operations rather than a persistent `.tasks` checkout.
The bundled launcher is `<plugin-root>/bin/tiber`; resolve `<plugin-root>`
relative to this skill file and prefer that launcher before probing `PATH`.

## Tool Selection

- Check for an installed `tiber` MCP server before using CLI commands.
- If the MCP tools are available, initialize the server with `tiber.init` only when setup is required, then use MCP tools for task reads and writes.
- If MCP tools are unavailable or fail to expose the needed operation, fall back to the bundled `tiber` CLI.
- Use `<plugin-root>/bin/tiber` before any `PATH` fallback.
- Offer `tiber install-bin --target-dir ~/.local/bin --dry-run` on setup or
  upgrade. Run `--apply` only after explicit user approval.

## Rules

- Never mutate a repo merely because the plugin is installed or a session starts.
- Run `tiber init` only for explicit setup or when a requested task operation
  needs an initialized board.
- Use CLI/MCP writes, not direct edits to `.tasks` files or `order.md`.
- Dashboard mode is read-only; all writes go through CLI or stdio MCP tools.
- Invoke the `tiber:new-task` skill for quick backlog capture when the user
  wants a new task recorded from chat. That skill writes only through structured
  Tiber MCP tools and leaves the task in `backlog` unless the user explicitly
  asks to start work immediately.
- When you open or discover a PR/MR for an in-progress task, update the task
  with `tiber update <ref> --pr-mr-url <url> --pr-mr-status <status>`, and keep
  `pr_mr_status` current as checks/review/merge state changes. Use stable
  lowercase statuses such as `draft`, `open`, `review-required`,
  `checks-pending`, `checks-passing`, `checks-failing`, `approved`, `merged`,
  `closed`, or `blocked`.
- Before actively working on an existing Tiber task, move it to `in-progress`
  with `tiber transition <ref> in-progress`; do not leave active work in the
  backlog as an informal reservation.
- Treat write-sync conflicts as hard failures: do not force push, choose local,
  or silently overwrite `tasks`. Preserve both sides, resolve deliberately, then
  rerun `tiber sync`.
- Before any task-board health claim, run and name `tiber validate --fix`.
  Safe autofixes are misplaced claims, missing reciprocal links, and `order.md`
  reconciliation. Dangling refs and dependency cycles are report-and-resolve.
- `claim:` is valid only on in-progress tasks. Backlog claims are invalid, not
  reservations; use `tiber transition <ref> in-progress`.
- For repo integration, run only `tiber scaffold repo --dry-run`, show the
  planned `.gitignore`, hook/workflow, trailer workflow, and optional
  `justfile` changes, then stop until explicit approval. "No follow-up
  questions" is not approval to apply.

## Commands

```shell
tiber init
tiber create "Task title"
tiber list
tiber show <task-ref>
tiber metadata <task-ref>
tiber next
tiber transition <task-ref> <status>
tiber prioritize <task-ref> --before <task-ref>
tiber link <task-ref> blocks <task-ref>
tiber unlink <task-ref> blocks <task-ref>
tiber subtask add <task-ref> "Subtask title" --after s1,s2
tiber update <task-ref> --summary "..."
tiber update <task-ref> --pr-mr-url <url> --pr-mr-status checks-pending
tiber acceptance add <task-ref> "Observable condition"
tiber note add <task-ref> "Progress note"
tiber validate --fix
tiber close-from-trailers
tiber mcp stdio
tiber dashboard serve
```
