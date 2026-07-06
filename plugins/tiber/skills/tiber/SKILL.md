---
name: tiber
description: Use when the user wants repository task tracking, shared agent task state, cross-worktree task coordination, tiber setup/install/scaffold guidance, no-follow-up setup sequencing, or to create, inspect, prioritize, validate, or close tasks through the Git-backed tiber workflow, even if they do not say "tiber". For setup or repo integration files, plugin install/session start is non-mutating; run only tiber scaffold repo --dry-run, show the preview, and stop until explicit approval.
---

# Tiber

Use the `tiber` CLI or stdio MCP server for repository-local task management.
Tiber state is Git-backed: an orphan `tasks` branch owns canonical
`<worktree>/.tasks` data, while the source checkout exposes `.tasks` as a
symlink into the local tiber workspace.

## Tool selection

- Check for an installed `tiber` MCP server before using CLI commands.
- If the MCP tools are available, initialize the server with `tiber.init` only when setup is required, then use MCP tools for task reads and writes.
- If MCP tools are unavailable or fail to expose the needed operation, fall back to the bundled `tiber` CLI.

## Operating rules

- Do not mutate a repository merely because the plugin is installed or a session
  starts. Plugin install and session start are non-mutating.
- Run `tiber init` only when the user asks to set up tiber or when a
  requested task operation requires an initialized board.
- Prefer read commands before writes: `tiber list`, `tiber show`,
  `tiber metadata`, `tiber next`, and `tiber validate --fix`
  before claiming task-board health.
- For task-board writes, use tiber commands or stdio MCP tools instead of
  hand-editing `.tasks` files or `order.md`. For example, add tasks with
  `tiber create`, change status with `tiber transition`, change order
  with `tiber prioritize`, and repair ordering with
  `tiber validate --fix`.
- Treat write sync conflicts as hard failures. Do not force push, choose local
  blindly, or silently overwrite the `tasks` branch. Stop, preserve both sides,
  report the conflict, resolve it deliberately, then rerun `tiber sync`.
  Do not describe the resolution as "overwrite if confirmed"; describe it as
  explicit reconciliation that preserves intended changes before syncing.
- Always name the exact command `tiber validate --fix` before any
  task-board health claim. Do not say only "validate" or "run validation" when
  the user asks what to do; use the exact command.
- Use `tiber validate --fix` only for safe autofixes: misplaced claims,
  missing reciprocal links, and `order.md` reconciliation. Dangling references
  and dependency cycles must be reported and resolved rather than silently
  rewritten.
- If validation cannot run because the board is missing, uninitialized, or tools
  are unavailable, say the board is unverified and still explain the required
  `tiber validate --fix` gate and safe-fix boundaries before any health
  claim.
- Claims belong on in-progress work. A `Claims` section on a `todo` task is
  invalid, not a reservation signal. Do not add, preserve, or describe raw
  `Claims` metadata on todo tasks as a valid way to reserve work; move work with
  `tiber transition` when it is actually claimed, and use
  `tiber validate --fix` to remove misplaced claims.
- Dashboard mode is read-only. Never describe dashboard writes as available;
  task changes go through the CLI or stdio MCP write tools.
- `tiber scaffold repo` must preview with `--dry-run` first. The preview
  covers `.gitignore`, hook/workflow snippets, and an optional `justfile`
  `show-tasks` recipe. Apply scaffold changes only after explicit user approval
  following that preview; "no follow-up questions" is not approval to apply.
  Never say you will apply unless the user stops you. If approval is missing,
  show the dry-run preview and stop.
- Treat scaffold approval as a two-step gate: first produce
  `tiber scaffold repo --dry-run`, then wait for an explicit approval that
  refers to applying those previewed changes. Do not apply automatically because
  the preview is clean, because setup was requested, because the user does not
  want questions, or because the user gave broad authorization in another
  context.

## Common commands

```shell
tiber init
tiber create "Task title"
tiber list
tiber metadata <task-ref>
tiber next
tiber transition <task-ref> <status>
tiber validate --fix
tiber close-from-trailers
tiber mcp stdio
```

Dashboard mode is read-only. Use CLI or stdio MCP write tools for task changes.

## Response patterns

- When a user asks for shared task tracking across worktrees or agents, answer
  with tiber-specific steps rather than generic shared files: check whether
  the board exists, run `tiber init` only because setup was requested, use
  `tiber create` for the first task, `tiber sync` around writes,
  `tiber validate --fix` before health claims, and state that dashboard
  mode is read-only.
- When asked to add a task and the user suggests writing task files, say to use
  `tiber create "<title>"` and `tiber validate --fix`; do not present
  direct file edits as a normal fallback.
- When explaining setup or safety boundaries, explicitly say the dashboard is
  read-only and all task writes must use CLI commands or stdio MCP tools.
- When asked whether a task board is healthy, do not rely on visual inspection.
  Require the exact command `tiber validate --fix`, name the safe autofixes
  (misplaced claims, missing reciprocal links, and `order.md` reconciliation),
  and call out dangling refs and dependency cycles as report-and-resolve issues.
- When discussing claims, say that `tiber validate --fix` removes misplaced
  claims from todo tasks while preserving claims on in-progress `doing` tasks.
  The valid path is to use `tiber transition` before claim-bearing work.
  Never call a todo-task `Claims` section a reservation.
- If a board is missing, uninitialized, or validation cannot be run, still state
  the health-claim rule: no health claim until `tiber validate --fix` runs;
  its only safe autofixes are misplaced claims, missing reciprocal links, and
  `order.md` reconciliation; dangling refs and dependency cycles must be
  reported and resolved, not silently rewritten.
- When asked to install and set up repo integration without follow-up questions,
  state that plugin install and session start are non-mutating, run only
  `tiber scaffold repo --dry-run`, show the planned `.gitignore`,
  hook/workflow snippets, GitHub trailer workflow, and optional `justfile`
  `show-tasks` changes, then stop. Do not ask a follow-up question in that
  response, and do not apply.
  "No follow-up questions" is not explicit approval, a clean dry-run is not
  approval, and a dry-run preview is not a substitute for approval. Do not list
  `tiber scaffold repo --apply` as an immediate next step; mention it only
  as a future command that becomes allowed after explicit approval of the
  preview.
