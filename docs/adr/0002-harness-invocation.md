# ADR-0002: Resolving a side-quest's harness invocation

## Status

Accepted

## Date

2026-07-01

## Context

A launched side-quest's goal session is driven by a `session_command`: a shell
command run (via `sh -c`) inside its worktree, with the goal exposed through
the `SIDEQUEST_GOAL` environment variable (see `session.rs`). In production,
that command was sourced _only_ from the `SIDEQUEST_SESSION_COMMAND`
environment variable read by `sidequest-mcp` at startup -- and nothing set it:
the plugin's `.mcp.json` sets no environment variables, and `sidequest.toml`'s
`[harness]` table only recorded a harness _label_ (`default`, `allow_cross`)
used for cross-harness gating, never an actual invocation.

The result: every real side-quest launched through the Claude Code plugin
skipped its goal session entirely (`quest::execute` only ran it
`if let Some(command) = session_command`), fell through to delivery logic that
ran regardless, and was marked `Delivered`/`Done` having made zero commits --
silently. Worse, if the session _did_ run and failed, the error propagated via
`?` out of `execute` before the registry was ever updated for that run, so a
failed side-quest was stuck at `Running` forever with no explanation, since
nothing reads the detached worker's own exit code or stderr (`spawn_worker`
nulls its stdio).

Both `claude` and `codex` support a fully headless, unattended invocation
(`claude --print ... --dangerously-skip-permissions`, `codex exec
--dangerously-bypass-approvals-and-sandbox ...`), but neither commits its
changes to git on its own initiative -- delivery assumes commits exist, so the
prompt itself must instruct the harness to commit.

## Decision

- `sidequest.toml` gains `[harness] command`: an explicit, project-level
  session-command override (`config::harness_command()`).
- `sidequest-core::harness::default_session_command(harness: &str)` supplies a
  built-in, pure (no I/O) template for known harness names (`claude`, `codex`)
  today. Each template passes the goal via `"$SIDEQUEST_GOAL"` (never
  string-interpolated -- the goal is untrusted prose that may contain quotes,
  apostrophes, and newlines) and explicitly instructs the harness to commit
  its work when finished.
- `quest::execute` resolves the effective command with this precedence:
  1. An explicit override (`--session-command`, still used by the CLI/tests).
  2. The project's `sidequest.toml` `[harness] command`.
  3. The built-in default for the side-quest's `harness` (recorded at launch).
  4. None of the above resolves -> the side-quest is marked `Failed` with a
     `detail` naming the unresolved harness, rather than silently `Done`.
- `quest::execute` never propagates a session or delivery failure as a Rust
  `Err` out of the function; every code path ends by writing a final registry
  state (`Delivered`, `Done`, `DoneNoChanges`, or `Failed` with `detail`).
  Nothing reads this worker process's exit code, so the registry is the only
  place an outcome can be observed.
- After the session runs, `deliver::has_new_commits` checks whether the branch
  actually gained commits relative to the project's current `HEAD` before
  attempting delivery. No new commits -> `DoneNoChanges`, distinct from `Done`
  (which means real work was left undelivered on the branch on purpose).

## Consequences

### Positive

- A side-quest launched via the plugin today actually invokes a real harness
  by default, with zero required configuration for `claude`/`codex` projects.
- Failure is always observable in the registry (`Failed` + `detail`) instead of
  a side-quest hanging at `Running` or silently reporting completion.
- `DoneNoChanges` distinguishes "the session ran but did nothing" from "real
  work is sitting on the branch, undelivered" -- the exact ambiguity that
  produced the original bug report.
- The core (`sidequest-core::harness`) stays pure; only the _resolution_
  precedence and the actual `sh -c` spawn live in the imperative shell.

### Negative

- Built-in templates hardcode two harnesses; a third harness requires either a
  code change here or a project-level `[harness] command` override.
- `--dangerously-skip-permissions` / `--dangerously-bypass-approvals-and-sandbox`
  run the harness fully unattended with no human approval gate. This is
  inherent to running a background side-quest at all (there is no human to
  approve anything), not unique to this decision, but it means a project
  adopting sidequest is trusting the harness's own judgment end to end.

## Alternatives Considered

### Require every project to set `[harness] command` explicitly

Rejected: defeats the purpose of a plugin that should work out of the box for
the two harnesses it already knows about; explicit configuration remains
available as an override for anything else.

### Silently retry or fall back to `Done` when no command resolves

Rejected: this is exactly the silent-failure behavior being fixed. An
unresolvable harness must be a loud, inspectable `Failed` state.

### Have `quest::execute` keep propagating errors via `?`

Rejected: nothing consumes this worker's exit code (`spawn_worker` nulls its
stdio and nobody waits on it), so an `Err` here is indistinguishable from a
process that never ran -- the side-quest would still show `Running` forever.

## Revisit when

A third harness needs a built-in default, or a harness ships a native
"commit on completion" mode that removes the need for the prompt-appended
commit instruction.

## Related

- ADR-0001 (overall architecture)
- ADR-0003 (session inspectability)
