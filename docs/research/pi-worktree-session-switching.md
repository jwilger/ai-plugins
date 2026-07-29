# Pi worktree session-switching research

Date: 2026-07-29

> Historical note: this research informed ADR-0005 and ADR-0006. Live outcomes
> and Pi 0.82.1 source later proved that `sendUserMessage()` deliberately skips
> extension-command dispatch; private transition text reached ordinary chat.
> ADR-0007 therefore supersedes the selected session-replacement design with
> session-persistent logical routing. The comparison below is retained as the
> evidence available when the earlier decisions were made.

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

## Updated validation and direction

Pi 0.82.1 exposes `switchSession()` only on manually invoked command contexts.
Tools receive `ExtensionContext`, and `sendUserMessage()` invokes `prompt()`
with command/template expansion disabled. Upstream issues 4754, 5912, 6010,
and 6574 confirm that deferred tool-to-command session mutation is not a
supported extension operation. Both automatic switch and finish reproduced the
same failure: their private `--automatic` command appeared as model-visible user
chat.

The desired behavior is instead the logical-routing pattern represented by the
season179 and rezamonangg candidates: keep the host cwd stable and route every
operation. Development-system closes the gaps identified in the original
comparison by centralizing one persisted authority, mutating all supported
built-in path tools, routing bash and user bash, and resolving status, guards,
review children, and component MCP calls through the same path. It explicitly
does not claim to reload Pi-native resources or provide a hostile shell sandbox.
See ADR-0007.

## Historical conclusion and attribution

The original investigation selected `@narumitw/pi-worktree` as the reference
for a user-invoked, true session replacement and implemented that design in
ADR-0005. No third-party package, runtime dependency, or source was installed,
vendored, or copied. The later automatic-command premise in ADR-0006 proved
false under live Pi 0.82.1 outcomes. ADR-0007 is the current implemented
contract; this document retains the candidate evidence and attribution only.
