# Development System

The development system is one configured development workflow for **Pi
(primary)**, **Claude Code (secondary)**, and **Codex (tertiary)**. All harnesses
load the same eight canonical public skills. Pi additionally loads a first-party
TypeScript extension for deterministic status, setup, path, worktree, delivery,
CI-recovery, retained-component, and final-review behavior.

## Install in Pi

Pi packages execute trusted code with the user's full operating-system
permissions. Review this package and trust only a source you control.

### GitHub Packages

Published releases use `@jwilger/development-system-pi` in GitHub Packages. The
package currently has restricted visibility, so authenticate npm with a classic
personal access token carrying `read:packages`, then install it through Pi's npm
package source:

```shell
npm login --scope=@jwilger --auth-type=legacy \
  --registry=https://npm.pkg.github.com
pi install npm:@jwilger/development-system-pi
```

Pin a release with
`pi install npm:@jwilger/development-system-pi@<version>`. Use
`pi update npm:@jwilger/development-system-pi` for an unpinned install and
`pi remove npm:@jwilger/development-system-pi` to remove it. Never commit the
resulting npm credential.

### Local checkout

From a clean local checkout, run the one reproducible bootstrap and install the
local package:

```shell
nix develop -c scripts/bootstrap-pi-package.sh
PI_OFFLINE=1 pi install ./plugins/development-system
```

The bootstrap restores pinned root dependencies inside the repository, validates
release metadata and the explicit Pi resource inventory, and verifies both
bundled binaries for the current supported target. Pi's local-path installation
references this package directory; it does not copy it. Pulling a package update
therefore makes it available to the next Pi process, or to `/reload` when the
resource is reload-safe.

On first use in a project, approve Pi's project-trust prompt only after reviewing
project-local settings and resources. Noninteractive callers must use an
explicit trust decision; the repository eval runner uses `--approve` only for
its owned disposable fixtures. The extension itself has full code-execution
privileges.

Remove a local-path installation with:

```shell
pi remove ./plugins/development-system
```

Use `pi list` and `pi config` to inspect or disable installed resources.

### Canonical Pi support inventory

This block is generated from `.agents/plugins/pi-support.json`; do not edit it
by hand.

<!-- pi-support-inventory:start -->

- **development-system** (`./plugins/development-system`)
  - extension: `./extensions/development-system/index.ts`
  - public skills (8): `agentic-systems`, `delivery`, `development-workflow`, `engineering-standards`, `eval-case-reporting`, `setup`, `tasks`, `worktrees`
  - bundled component entry points: `./bin/development-discipline-mcp`, `./bin/tiber`

<!-- pi-support-inventory:end -->

## Configure a project

Run trusted setup from the repository's primary checkout in the local Pi TUI:

```text
/development-system-setup
/development-system-setup --delivery pull-request --enable agentic-systems
```

The command resolves the primary checkout, shows the complete dry-run preview,
binds approval to repository identity, HEAD, tracked state, config identity,
arguments, and preview, then rechecks that binding immediately before applying.
Only local-TUI confirmation can mutate. RPC, print, and JSON modes stop after the
preview with `development_system.setup_confirmation_required`.

The compatible direct CLI remains:

```shell
development-system setup --project . --preset personal-trunk --dry-run
development-system setup --project . --preset personal-trunk --apply --yes
```

Both paths delegate to the same setup implementation, which rejects linked
worktrees and rolls back the config and index if its single initialization
commit fails.

`.development-system.toml` remains authoritative. The default preset is
direct-to-trunk delivery with linked worktrees and Tiber. Optional features are
`agentic-systems` and `eval-case-reporting`. Existing schema-version 1 files
remain compatible. Optional Pi final-review routes use:

```toml
[pi.review_models]
bounded_helper = "openai-codex/gpt-5.6-luna"
substantive_worker = "openai-codex/gpt-5.6-terra"
strong_reviewer = "openai-codex/gpt-5.6-sol"
strong_worker = "openai-codex/gpt-5.6-sol"
```

## Autonomous goals

Pi owns one session-branch-scoped autonomous goal at a time:

```text
/goal
/goal status
/goal [--tokens 200k] [--turns 25|unlimited] <objective>
/goal pause
/goal resume [--tokens 300k] [--turns 10|unlimited]
/goal clear
```

The default is 25 automatic responses. `unlimited` must be explicit; an optional
token budget uses provider-reported usage and may overshoot by one call. Goals
pause rather than claim success at response, token, repeated-output, provider,
interruption, restricted-terminal-tool, or collision boundaries. Resume rotates
the goal ID and safety epoch while retaining consumed token usage. State is
stored only in Pi custom entries on the current session branch, so reopening that
session restores it while a new session does not inherit it.

Only `goal_complete` with the exact current goal ID and direct completion and
verification evidence can complete a goal. `goal_blocked` requires the same
external blocker across at least three attempts and concrete evidence that user
or external action is required. Plain assistant text, stale turns, delayed
continuations, difficulty, incomplete work, and recoverable failures cannot
terminate successfully. Continuations are extension-authored custom messages
dispatched only after Pi's settled and idle boundary.

## Status and diagnostics

Use `/development-system-status` in Pi. Headless callers use the deterministic
non-model entry point:

```shell
plugins/development-system/bin/development-system-pi status \
  --project . --mode json
```

Status reports configuration presence, delivery mode, enabled features, primary
or linked checkout identity, bundled component availability, active mode
limitations, and actionable typed errors. Startup runs the same compatibility
doctor used by the existing harness hooks. A model-callable read-only status
tool returns only a concise task-facing summary by default.

## Capability matrix

| Capability                          | Pi (primary)                                                | Claude Code (secondary)       | Codex (tertiary)              |
| ----------------------------------- | ----------------------------------------------------------- | ----------------------------- | ----------------------------- |
| Eight shared public skills          | Canonical files                                             | Same canonical files          | Same canonical files          |
| Setup/doctor core                   | Native command/tool and CLI                                 | Hook/CLI adapter              | Hook/CLI adapter              |
| Trusted consequential approval      | Local TUI, preview-bound                                    | Unavailable                   | Unavailable                   |
| Bounded autonomous goal mode        | Session-scoped `/goal` with guarded terminal tools          | Unavailable                   | Unavailable                   |
| Generic write/edit worktree guard   | Extension event-enforced                                    | Hook-enforced where supported | Hook-enforced where supported |
| Default model bash guard            | Extension event-enforced in TUI/print/JSON model-tool paths | Instruction/hook boundary     | Instruction/hook boundary     |
| Direct RPC bash                     | Unsupported for guarded execution                           | N/A                           | N/A                           |
| Protected metadata/secret paths     | Guarded built-in path tools; search output mediated         | Instruction/hook limits       | Instruction/hook limits       |
| Delivery-mode and force-push policy | Extension event-enforced                                    | Hook/command-enforced         | Hook/command-enforced         |
| Tiber CI-recovery hold              | Extension event plus authoritative Tiber state              | Hook/skill plus Tiber state   | Hook/skill plus Tiber state   |
| Tiber tools                         | Feature-aware native Pi bridge                              | Plugin MCP                    | Plugin MCP                    |
| Final-review coordinator            | Native Pi bridge to authoritative Rust MCP                  | Plugin MCP                    | Plugin MCP                    |
| Fresh final-review children         | Isolated Pi child sessions with attestation                 | Harness agents                | Harness agents                |
| Agent definitions                   | Canonical source with generated adapter                     | Generated Markdown            | Generated TOML                |

The populated-secret claim covers the characterized built-in guarded tool
composition. It does not claim protection from an intentionally unmediated
shell command, third-party tool, owner bypass, or malicious same-UID process.
Unknown extension tools visibly downgrade the guarded-composition status.

## Supported targets and compatibility

The package supports Pi `>=0.82.0 <0.83.0` and release canaries pin Pi `0.82.1`.
Bundled Tiber and development-discipline binaries are verified for:

- x86_64 Linux;
- aarch64 Linux;
- x86_64 macOS; and
- Apple-silicon macOS.

Other targets return `development_system.unsupported_platform` before component
invocation. Cargo fallback is development-only and is not part of installed
package behavior.

## Contributor sources and validation

Canonical sources are:

- package version and Pi resources: `package.json` and
  `.agents/plugins/pi-support.json`;
- shared skill prose: `skills/*/SKILL.md`;
- extension semantics: `extensions/development-system/core/`;
- harness/process adapters: `extensions/development-system/adapters/`;
- review agents: `components/development-discipline/agent-sources/review-agents.json`;
- retained authority: the Tiber and development-discipline Rust components.

Regenerate or validate adapters with:

```shell
node scripts/sync-development-system-metadata.mjs --write
node scripts/generate-development-system-agents.mjs --write
node scripts/generate-pi-support-docs.mjs --write
node scripts/sync-development-system-metadata.mjs --check
node scripts/generate-development-system-agents.mjs --check
node scripts/generate-pi-support-docs.mjs --check
just pi-extension
just pi-clean-canary
```

Generated agent Markdown/TOML files carry no independent semantic authority.
Do not edit them without regenerating their canonical source.
