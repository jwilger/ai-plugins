---
name: tiber
description: Use whenever Tiber or origin/tiber is mentioned, including repository task tracking, EventCore publication conflicts, ambiguous pushes, tiber sync recovery, force-push safety, cross-worktree task-board coordination, setup, and task workflows.
---

# Tiber

Use Tiber for repository-local task boards backed by EventCore and eventcore-fs
on the shared Git `tiber` branch. Markdown task files are no longer persistent
state. Pushed-CI recovery belongs to Development Discipline's independent
workflow coordinator, not this task board.
The bundled launcher is `<plugin-root>/bin/tiber`; resolve `<plugin-root>`
relative to this skill file and prefer that launcher before probing `PATH`.

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
- Task reads and writes may run from a coordination-only primary checkout when
  Git reports it as bare but explicitly declares the source root through
  `core.worktree`; relative values resolve from the Git directory. Refuse a true
  bare repository or another context without an explicit worktree root with
  actionable repository-context guidance.
- Run `tiber init` only for explicit setup or when a requested task operation
  needs an initialized board. Legacy `.tasks` state is ignored rather than
  migrated; recreate any worthwhile historical tickets manually.
- Established-repository scaffold is a safety preflight, not file generation:
  verify the active executable hook-manager dispatcher, require pinned workflow
  and Tiber revisions with locked dependencies and only `contents: write`,
  target the repository's publication branch, and refuse generated publication
  when repository policy requires signing but no repository-owned automation
  supplies the key.
- Use CLI/MCP writes, not direct edits to the event store or authoritative Git
  ref. Legacy `.tasks` files and `order.md` are ignored history, not Tiber state.
- The dashboard can reorder backlog priority, which does not change capacity.
  It has no create or status-transition route. Admission writes go through CLI
  or stdio MCP tools and share the same capacity enforcement.
- Start a dashboard with `tiber dashboard serve --open` when the user wants it
  opened in a browser. An unconfigured launch selects an available localhost
  port and prints the complete URL. Repeated launches for the same project
  reuse its healthy server without opening another browser window. Use
  `--port <port>` only when the user requests a fixed port; a conflicting
  running instance or occupied requested port must be reported, not bypassed
  with a second project server.
- Invoke the `tiber:new-task` skill for quick backlog capture when the user
  wants a new task recorded from chat. That skill writes only through structured
  Tiber MCP tools and leaves the task in `backlog` unless the user explicitly
  asks to start work immediately.
- Before admitting a candidate, use `tiber.search` (CLI:
  `tiber search "<outcome terms>"`) across all statuses, including `done` and
  `abandoned`. Compare historical matches with the candidate. Combine genuinely
  overlapping work or reject a duplicate; create only when the candidate is
  materially distinct. Discovery alone does not create an obligation.
- When you open or discover a PR/MR for an in-progress task, update the task
  with `tiber update <ref> --pr-mr-url <url> --pr-mr-status <status>`, and keep
  `pr_mr_status` current as checks/review/merge state changes. Use stable
  lowercase statuses such as `draft`, `open`, `review-required`,
  `checks-pending`, `checks-passing`, `checks-failing`, `approved`, `merged`,
  `closed`, or `blocked`.
- Before actively working on an existing Tiber task, move it to `in-progress`
  with `tiber transition <ref> in-progress`; do not leave active work in the
  backlog as an informal reservation.
- When `.tiber.toml` sets `[backlog].max_queued`, only `backlog` tasks count.
  The active `in-progress` task does not count. Creating, reopening, or moving
  work into `backlog` refuses once the limit is full across CLI and MCP.
- Treat `tiber.backlog_capacity_exceeded` as an admission decision, not a
  retryable sync failure. Do not create overflow, icebox, shadow, or hidden
  work. Require the user to replace a lower-value queued task, combine
  genuinely overlapping work, or reject the candidate.
- A confirmed expected stream-version mismatch is an optimistic-concurrency
  conflict: Tiber refreshes the authoritative branch and retries with the same
  invocation identity and inputs. An ambiguous publication outcome means durable
  publication was not confirmed or rejected; stop all mutations and use
  `tiber sync` to reconcile that exact pending invocation. Never force-push or
  rewrite `tiber`.
- Treat `close-from-trailers` as successful only when it synchronizes the
  authoritative board, resolves every `Closes:` line from the current `HEAD`
  commit, prints `closed <task-id>` for every requested task, and leaves every
  named task no longer open. A missing or invalid task, synchronization
  conflict, or publication/push failure must produce a specific diagnostic and
  a nonzero exit; never accept exit zero or empty output alone as closure
  evidence.
- Before any task-board health claim, run and name `tiber validate --fix`.
  Safe autofixes include misplaced claims and missing reciprocal links.
  Dangling refs and dependency cycles are report-and-resolve.
- `claim:` is valid only on in-progress tasks. Backlog claims are invalid, not
  reservations; use `tiber transition <ref> in-progress`.
- For repo integration, run only `tiber scaffold repo --dry-run`, show the
  planned hook/workflow, trailer workflow, and optional
  `justfile` additions, already-configured integrations, and conflicts, then
  stop until explicit approval. Evaluate hooks and workflows independently: an
  equivalent existing workflow suppresses only the generated
  workflow, and an equivalent existing hook suppresses only the generated hook.
  Preview any distinct missing integration. Legacy `.tasks`, `tasks`, and
  `tiber-coordination` state is ignored and never migrated automatically.
  Verify that Git's active executable `post-commit` hook
  dispatches the proposed Tiber snippet; a file that the active hook manager
  never invokes is not installed automation. Generated GitHub automation must
  pin both checkout and Tiber source revisions, install locked dependencies,
  target the repository's publication branch, declare only `contents: write`,
  and refuse signed-publication policy unless repository-owned automation
  supplies a signing key. Apply
  refuses ambiguous integration-file replacements; use `--replace-conflicts`
  only after the user explicitly chooses to replace every reported conflict.
  "No follow-up questions" is not approval to apply.

## Pushed-CI Incident Coordination

Tiber does not coordinate CI recovery. Load Development Discipline's
`ci-failure-follow-up` skill and use `workflow.ci_recovery.*`; its independent
workflow authority applies to projects with and without Tiber.

## Commands

```shell
tiber init
tiber codex-sandbox --dry-run
tiber create "Task title"
tiber list
tiber list --status done
tiber search "outcome terms"
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
tiber dashboard serve [--open] [--port <port>]
```

Pass multiline summary and context values directly through the structured CLI
or MCP update surface. Preserve actual line breaks and literal backslashes; do
not flatten text or substitute literal `\n` sequences. If Tiber rejects an
embedded heading reserved by the task schema, follow its recovery guidance to
demote or rename that heading.
