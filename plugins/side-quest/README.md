# side-quest

Launch backgrounded "side-quests": each implements a change in its own isolated
git worktree and delivers the result per the project's `sidequest.toml`.

## Harnesses

- **Claude Code** — provides the `/side-quest` command and the `sidequest` MCP
  server (this directory).
- **Codex** — parallel manifest planned (the MCP server and command are
  harness-agnostic; Codex consumes the same MCP server).

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
