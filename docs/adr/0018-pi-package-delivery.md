# ADR-0018: Deliver Development System as a pi package from the same plugin root

## Status

Accepted

## Date

2026-10-05

## Context

Development System is a Codex plugin. Its deterministic behavior already lives
outside Codex-specific files: the two MCP servers are plugin-root launchers in
`mcp.json`, binary repair lives in `lib/installed-binary.sh`, and the
session-start conflict check lives in `bin/development-system`. Only
`hooks/codex.json` and the `agents/*.toml` subagent definitions are tied to the
Codex runtime. ADR-0014 required a new ADR and harness behavior evidence before
adding a harness. ADR-0016 lets other harnesses use the portable components
without a compatibility promise.

[pi](https://github.com/earendil-works/pi) loads packages declared by a
`package.json` `pi` key. A package can contribute skills and TypeScript
extensions. Extensions can register MCP servers, subscribe to `session_start`,
add slash commands, and run processes through `pi.exec`. A local package can be
installed in place with `pi install ./plugins/development-system`.

## Decision

Ship the pi package from the existing plugin root rather than from a copy:

- `plugins/development-system/package.json` declares `pi.skills: ["./skills"]`
  and `pi.extensions: ["./pi/extension.ts"]`. Component-internal `skills/`
  directories stay out of the manifest because they duplicate retained
  contracts.
- `pi/extension.ts` is a policy-free adapter. It registers every server in the
  plugin's `mcp.json` with the launcher path resolved against the plugin root,
  so pi runs the same self-repairing launchers as Codex. It registers tools with
  `direct` exposure for parity with Codex. It leaves the MCP working directory
  unset because the Tiber launcher derives the project from it.
- Session start calls the shared CLI as
  `bin/development-system session-start --harness pi`. The call runs in the
  background because binary repair can download a release bundle. Warnings and
  failures are shown as pi notifications. `/development-system doctor` runs the
  same check on demand.
- The shared CLI owns harness-specific conflict rules. For pi, it reports
  project or user `mcp.json` entries that shadow the bundled server names and
  settings that disable pi's built-in MCP support. It does not flag other
  installed pi packages. Pi has no plugin-cache layout equivalent to Codex, and
  a package-presence check would mostly produce noise.
- `package.json` carries the same version as `plugin.json`. The bump script
  synchronizes it, and manifest validation rejects version drift or missing
  `pi` resource paths.
- The adapter is type-checked with TypeScript against local structural types,
  so this repository needs no pi runtime dependency. Node's built-in test runner
  exercises it against a fake pi API in `just ci` and the pre-commit gate.

Subagents, harness-neutral skill wording, pi behavior evals, and npm publication
are separate later increments. Until pi behavior evidence exists, the pi package
is experimental and Codex remains the only supported harness.

## Consequences

### Positive

- One copy of skills, MCP launchers, binaries, and conflict checks serves both
  harnesses. Every deterministic rule still lives in the shared shell and Rust
  core.
- The pi adapter is small, typed, and covered by deterministic tests.
- Version drift between the Codex and pi manifests fails validation.

### Negative

- Skills still contain Codex-specific instructions, such as
  `.codex/config.toml` sandbox setup, until a later increment makes them
  harness-neutral.
- Codex subagents (`agents/*.toml`) are not yet available in pi. Pi subagent
  managers discover agents from fixed directories rather than a registration
  API.
- The structural pi types can drift from pi's real API. Only a real pi smoke run
  or pi behavior eval detects that drift.
- Users who replace pi's built-in MCP support with another extension do not get
  the bundled servers. The doctor check reports only the documented built-in
  disable setting.

## Alternatives Considered

### Separate pi plugin directory

Rejected. Duplicating skills or launchers would split the source of truth that
ADR-0016 consolidated.

### Reimplement the session-start check in TypeScript

Rejected. It would duplicate deterministic policy in a second language. The
adapter shells out to the existing CLI instead.

### Depend on pi's published type packages

Rejected for now. A local structural interface keeps the repository free of a
pi runtime and its dependency tree. Revisit if the adapter grows beyond
registration, one event, and one command.

## Revisit when

- Pi behavior evals exist and the package is ready for an npm release.
- Pi adds a package-level subagent registration API.
- The adapter needs pi APIs beyond MCP registration, `session_start`, commands,
  `exec`, and notifications.

## Related

- ADR-0014
- ADR-0016
