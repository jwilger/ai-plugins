# side-quest

Launch backgrounded "side-quests": each implements a change in its own isolated
git worktree and delivers the result per the project's `sidequest.toml`.

## Harnesses

- **Claude Code** — provides the `/side-quest` command and the `sidequest` MCP
  server (this directory).
- **Codex** — provides the `side-quest` skill and the same `sidequest` MCP
  server. The skill maps natural-language requests to the MCP `launch`, `list`,
  and `logs` tools.

## Requirements

The control-plane binary must be installed and on `PATH`:

```shell
cargo install sidequest
```

This provides `sidequest-mcp` (the MCP server, used by this plugin) and the
`sidequest` CLI.

## Usage

```
/side-quest The action buttons at the bottom of this section look bad and are confusing. Improve them.
```

The side-quest runs in its own worktree (default `./.worktrees/`, configurable)
and delivers its work according to `sidequest.toml`, e.g.:

```toml
[delivery]
mode = "local-merge"
```

## Which harness actually does the work

A side-quest's goal session is driven by a real headless invocation of a
harness. `claude` and `codex` have built-in default invocations, so most
projects need no configuration at all. To use a different command (a
different harness, extra flags, etc.), set an explicit override:

```toml
[harness]
command = "claude --print \"$SIDEQUEST_GOAL\" --dangerously-skip-permissions"
```

`$SIDEQUEST_GOAL` must stay as a literal environment-variable reference (never
paste the goal text directly into the command) — the shell substitutes it
safely as a single argument, however the goal is phrased. If no override is
configured and the targeted harness has no built-in default, the side-quest
fails immediately with a clear reason instead of silently reporting success
having done nothing.

## Checking on a running side-quest

- `list` shows every side-quest's branch and state: `running`,
  `awaiting-input`, `delivered`, `done`, `done-no-changes` (the session ran but
  produced no commits, so there was nothing to deliver), or `failed` (with a
  `detail` explaining why).
- `logs` reads back a side-quest's captured session output (its harness's
  stdout/stderr, written live as it runs), tail-first. Call it with the
  side-quest's branch to see what it's actually doing, whether it's still
  running or already finished.
