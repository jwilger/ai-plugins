---
name: tiber
description: Use when repository work needs Tiber task tracking, shared agent task state, setup/scaffold, task workflows, or sync-conflict recovery. Conflicts are hard stops: never force overwrite; name `tiber conflict show <path>` and `tiber conflict resolve <path> --local|--remote`.
---

# Tiber

Use Tiber for repository-local task boards backed by Tiber-owned Git storage.
Treat that storage as an implementation detail: agents should interact through
Tiber CLI/MCP commands and the read-only dashboard, not host-repository task
files or storage paths. The bundled launcher is `<plugin-root>/bin/tiber`;
resolve `<plugin-root>` relative to this skill file and prefer that launcher
before probing `PATH`.

## Tool Selection

- Check for an installed `tiber` MCP server before using CLI commands.
- If the MCP tools are available, initialize the server with `tiber.init` only when setup is required, then use MCP tools for task reads and writes.
- If MCP tools are unavailable or fail to expose the needed operation, fall back to the bundled `tiber` CLI.
- Use `<plugin-root>/bin/tiber` before any `PATH` fallback.
- Offer `tiber install-bin --target-dir ~/.local/bin --dry-run` on setup or
  upgrade. Run `--apply` only after explicit user approval.
- During setup, or after a Tiber MCP write/sync fails because Git cannot write
  refs, objects, signed commits, or push credentials from the Codex sandbox,
  call `tiber.codex_sandbox_setup` or run `tiber codex-sandbox --dry-run`.
  Use case-by-case approval for raw Git prefixes. Persist approval only when the
  harness can scope it to the exact Tiber-internal operation, not merely to a raw
  `git` prefix. Then retry the same structured MCP operation. Do not ask the
  user to rerun an equivalent Tiber CLI command manually as the normal recovery
  path, and do not recommend running the whole Tiber MCP server outside the
  sandbox unless the narrow Git permissions are insufficient.

## Rules

- Never mutate a repo merely because the plugin is installed or a session starts.
- Run `tiber init` only for explicit setup or when a requested task operation
  needs an initialized board.
- Use CLI/MCP writes, not direct edits to Tiber-owned storage, task markdown
  files, or ordering files.
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
- Valid task statuses are `backlog`, `in-progress`, `done`, and `abandoned`.
  Treat `backlog` and `in-progress` as open work; use `done` for completed work
  and `abandoned` only when the task is intentionally dropped.
- Treat write-sync conflicts as hard failures: do not force push, choose local
  blindly, use any force-style overwrite path, or silently overwrite Tiber-owned
  task state. There is no "force it if the other side seems obsolete" exception.
  Preserve both sides, resolve deliberately, use `tiber conflict show <path>`
  when normal reads are blocked, choose the intended side with
  `tiber conflict resolve <path> --local` or
  `tiber conflict resolve <path> --remote`, include every conflict path in one
  resolve command when multiple conflicts are present, then rerun `tiber sync`.
- In any user-facing answer about a `tiber sync` conflict, name the exact
  diagnostic command/tool and resolver command/tool. If the path is not known
  yet, say to copy the diagnostic `<path>` from the sync error and run
  `tiber conflict show <path>` or `tiber.conflict_show`, then resolve with
  `tiber conflict resolve <path> --local|--remote` or `tiber.conflict_resolve`
  before rerunning `tiber sync`. That diagnostic path is not a normal task ref;
  do not invent it, use it with ordinary task commands, or treat it as host
  repository storage. Do not stop at generic "inspect and reconcile" guidance,
  and do not mention force overwrite as an acceptable fallback.
- If Tiber reports `task_blob_too_large`, treat it as a hard coordination stop.
  Do not create a duplicate task, route it through conflict tools, force-push,
  blindly overwrite shared task state, or hand-edit Tiber-owned storage as an
  ordinary agent workflow. Coordinate with a human/operator who can repair
  Tiber-owned state intentionally, shrink or remove the oversized task blob from
  `refs/heads/tasks` or `origin/tasks`, then rerun Tiber validation/sync.
- If Tiber reports `tasks_remote_rewritten`, treat it as a hard coordination
  stop. Inspect `origin/tasks` with a human/operator or a clean checkout before
  any recovery; do not force-push, recreate, or overwrite the shared tasks ref
  from local state.
- Before any task-board health claim, run and name `tiber validate --fix`.
  Safe autofixes are misplaced claims, missing reciprocal links, and ordering
  reconciliation. Dangling refs and dependency cycles are report-and-resolve.
- `claim:` is valid only on in-progress tasks. Backlog claims are invalid, not
  reservations; use `tiber transition <ref> in-progress`.
- For repo integration, run only `tiber scaffold repo --dry-run`, show the
  planned hook and workflow integration files, then stop until explicit approval.
  "No follow-up questions" is not approval to apply.

## Commands

```shell
tiber init
tiber codex-sandbox --dry-run
tiber create "Task title"
tiber list
tiber show <task-ref>
tiber metadata <task-ref>
tiber conflict show <path>
tiber conflict resolve <path> --local
tiber conflict resolve <path> --remote
tiber conflict resolve <path-a> --local <path-b> --remote
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
