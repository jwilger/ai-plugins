# Pi worktree session-switching research

Date: 2026-07-29

> Historical note: this research informed ADR-0005. ADR-0006 supersedes the
> user-initiation conclusion after confirming Pi's documented
> `sendUserMessage()` follow-up bridge into extension commands.

## Question

Can a Pi extension move an active conversation into another Git worktree
without restarting Pi, and is an available implementation safe enough to inform
development-system?

## Method and evidence limits

This review searched npm for Pi worktree packages, inspected package metadata
and download counts, queried public GitHub repository metadata, and read cloned
source without installing or executing third-party packages. Counts are a
2026-07-29 snapshot and will drift. npm downloads are not unique users, GitHub
stars are not a security endorsement, and monorepository signals cover more
than one package.

Primary sources:

- [npm search results for Pi worktree packages](https://www.npmjs.com/search?q=pi%20worktree)
- [`@narumitw/pi-worktree` package](https://www.npmjs.com/package/@narumitw/pi-worktree)
  and [source](https://github.com/narumiruna/pi-extensions/tree/main/extensions/pi-worktree)
- [`@season179/pi-worktree` package](https://www.npmjs.com/package/@season179/pi-worktree)
  and [source](https://github.com/season179/pi-ecosystem/tree/main/packages/pi-worktree)
- [`@rezamonangg/pi-worktree` package](https://www.npmjs.com/package/@rezamonangg/pi-worktree)
  and [source](https://github.com/rezamonangg/pi-worktree)
- [`@thisux/pi-worktree` package](https://www.npmjs.com/package/@thisux/pi-worktree)
  and [source](https://github.com/thisuxhq/pi-worktree)
- [`@pandi-coding-agent/pandi-worktree` package](https://www.npmjs.com/package/@pandi-coding-agent/pandi-worktree)
  and [source](https://github.com/andrestobelem/pandi-extensions/tree/main/extensions/pandi-worktree)
- [`@yassimba/pi-herdr-worktree` package](https://www.npmjs.com/package/@yassimba/pi-herdr-worktree)
  and [source](https://github.com/Yassimba/ai-setup/tree/main/plugins/herdr-worktree)
- [Pi 0.82 extension API](https://github.com/earendil-works/pi/blob/main/packages/coding-agent/docs/extensions.md),
  checked against the locally installed 0.82.1 declarations and runtime

## Candidate comparison

| Candidate                                  | Adoption and maintenance snapshot                                                                                                                                                                     | What “switch/open” actually does                                                                                                                                                                         | Security and fit assessment                                                                                                                                                                                                                                                                        |
| ------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `@narumitw/pi-worktree` 0.35.0             | 757 npm downloads in the preceding month; parent repository 239 stars, 36 forks, 18 contributors; npm modified 2026-07-28; MIT; six worktree test files containing 69 tests in the inspected checkout | Waits for idle, writes/forks a private target-cwd Pi session, then calls public `ctx.switchSession()`. Pi tears down and rebuilds the cwd-bound runtime while preserving the active conversation branch. | Best fit. Uses argv Git calls, revalidates selections, does not interpolate shell input, does not delete a worktree on switch failure, and avoids stale source context after replacement. Broad removal/prune features are unnecessary for development-system.                                     |
| `@season179/pi-worktree` 26.7.1            | 328 monthly npm downloads; parent repository 1 star and 1 contributor; npm modified 2026-07-10; MIT                                                                                                   | A startup flag creates a worktree, mutates built-in tool paths, and prefixes bash with `cd <worktree> &&`. It keeps Pi’s actual session cwd unchanged.                                                   | Rejected as the reference. Virtual routing does not naturally rebind arbitrary custom tools, context/resource loading, status, or project trust. Its shutdown path can offer force removal of a dirty worktree, which is outside our preservation policy.                                          |
| `@rezamonangg/pi-worktree` 0.1.3           | 104 monthly npm downloads; repository 1 star, 2 forks, 1 contributor; npm modified 2026-06-09; no license declared in npm or the inspected repository                                                 | Activates model-callable virtual routing, mutates file-tool paths, and replaces bash with a cwd-routed tool.                                                                                             | Interesting defense-in-depth path checks, but rejected. The actual Pi runtime remains in the source checkout; Pi-managed and temporary outside paths are exceptions; custom tools need separate routing; persisted virtual state expands recovery complexity; and absent licensing prevents reuse. |
| `@thisux/pi-worktree` 1.2.0                | 156 monthly npm downloads; repository 0 stars, 2 contributors; release 2026-07-22; MIT                                                                                                                | “Open” copies/displays the target path and tells the user to run `cd <path> && pi`.                                                                                                                      | Does not switch the current Pi runtime. Useful worktree CRUD, but it retains the relaunch workflow this change is intended to remove.                                                                                                                                                              |
| `@pandi-coding-agent/pandi-worktree` 0.2.7 | 1,591 monthly npm downloads; parent repository 2 stars, 2 contributors; npm modified 2026-07-11; MIT                                                                                                  | Its model tool creates or opens a new Supacode tab when available; otherwise it returns `cd <path> && pi`. Source explicitly says the current cwd never changes.                                         | More adopted by npm count, but not an in-process switch. It also supports force removal and copying ignored/untracked files, authorities we do not need.                                                                                                                                           |
| `@yassimba/pi-herdr-worktree` 0.2.1        | 348 monthly npm downloads; parent repository 3 stars, 1 contributor; npm modified 2026-07-16; MIT package in an Apache-2.0 repository                                                                 | Forks the session file, opens another Herdr pane running Pi in the worktree, and cleans up the old pane.                                                                                                 | Preserves conversation and avoids a manual relaunch, but still launches another process and requires Herdr plus Worktrunk. More process, environment, and terminal orchestration than needed.                                                                                                      |

Higher-download orchestration packages such as `pi-crew` were also inspected.
They create worktrees for child agents rather than rebind the active root Pi
session, so they do not answer this question.

## Selected reference

`@narumitw/pi-worktree` is the behavioral and architectural reference. Its key
insight is that a Pi process need not change its operating-system cwd. A Pi
**session** stores its own cwd, and a user-initiated extension command can use
Pi's public session-replacement API to rebuild the runtime around another cwd.
This is a true workspace replacement rather than command/path spoofing.

The implementation in development-system is repository-native and limited to
switching already registered worktrees. No third-party package or runtime
transitive dependency is added, and no third-party source is vendored. The MIT
reference permits learning from its public design; this document provides
attribution.

## Security review and resulting boundary

### Session and transcript handling

The active session-tree branch is serialized with Pi's `SessionManager` into a
new target-cwd session file using exclusive creation and mode `0600`. It is not
written into the worktree and contains no copied environment or authentication
store. Existing conversation content is intentionally preserved; the local TUI
confirmation makes that transfer explicit. A cancelled or failed replacement
retains the private session for recovery rather than risking destructive
cleanup.

### Authority and project trust

Only `ExtensionCommandContext` exposes `switchSession()`. Therefore the switch
is a user-initiated local-TUI command, not an LLM tool. A model can create a
worktree through the existing parsed semantic tool and can report the exact
switch command, but it cannot silently transfer the conversation. Pi owns
reloading cwd-bound resources, extensions, context files, and project trust in
the replacement runtime.

Headless JSON, print, and RPC callers fail closed and retain the explicit
new-process command as a fallback. This limitation is preferable to casting an
ordinary tool context to an unsupported command context or virtualizing only a
subset of tools.

### Path, race, and terminal handling

The selector resolves only to Git's registered worktree inventory. Exact path,
branch, or unique basename matches are accepted; ambiguous and control-bearing
selectors fail. After confirmation, path, branch, and HEAD identity are read
again before preparation. Target cwd is canonicalized before serialization and
verified after opening the prepared session. The replacement callback verifies
Pi's fresh cwd before reporting success. User-visible paths, branches, and
errors are stripped of terminal control sequences and bounded.

There is an unavoidable same-UID race between final Git revalidation and Pi
runtime replacement. This extension remains protection against ordinary model
and owner mistakes, not a same-UID operating-system sandbox.

### Process, command, and supply-chain behavior

Switching spawns no shell and no new Pi process. It runs no third-party install
script, copies no auth state, and adds no dependency. Worktree creation remains
argv-based, repository-root-contained, collision-safe, and non-destructive.
The switch command does not add removal, prune, force, branch deletion, commit,
or push authority.

## Implemented contract

- `/development-system-worktree-switch` opens a registered-worktree selector.
- `/development-system-worktree-switch <exact-branch-or-path>` selects directly;
  a basename is accepted only when unique.
- The local TUI waits for idle, confirms conversation transfer, revalidates Git
  identity, prepares and verifies a private target session, then invokes Pi's
  public session replacement API.
- The old extension context is never used after successful replacement.
- `development_system_worktree_list` and
  `development_system_worktree_create` return `switchCommand` as the preferred
  TUI handoff and retain `relaunchCommand` for headless use.
- The operating-system process cwd remains immutable; the complete Pi
  cwd-bound runtime is replaced. Command-level `cd` and `git -C` still do not
  grant workspace identity.
