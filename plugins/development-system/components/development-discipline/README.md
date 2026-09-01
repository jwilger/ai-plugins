# development-discipline

John's personal workflow plugin for development discipline. It packages the
workflow skills that should replace the upstream `superpowers` variants in day
to day work, tuned for this marketplace and personal reuse rather than public
generality.

## Skills

- `development-workflow` - inspects the current development phase and routes it
  to the smallest applicable specialist workflow without duplicating mechanics.
- `model-routing` - selects an explicit task-local model for bounded,
  substantive, and strong-responsibility work, with visible failure instead of
  silent fallback.
- `change-preflight` - classifies a requested change and records every affected
  project surface from repository evidence before implementation starts.
- `test-driven-development` - classifies whether RED applies, uses one failing
  behavior test when required, and defines safe remove-first handling for
  obsolete tests.
- `delivery-workflow` - routes delivery through repository-local instructions,
  after current user direction, supporting direct-to-trunk, PR/MR, and
  local-only work without inventing a pull request or letting a specialist skill
  change the selected mode, cadence, or evidence level; repairs default to new
  additive commits, every authored commit requires a rationale-bearing body,
  and amends require explicit case-by-case authorization.
- `ci-failure-follow-up` - evidence-based recovery that blocks unrelated
  work after a pushed CI failure until a replacement run succeeds.
- `rationale-commit-messages` - Conventional Commit subjects with a required
  body that explains why the change is necessary, without treating message
  authoring as authorization to amend an existing commit.
- `verification-before-completion` - evidence-before-claims discipline tied to
  the actual claim scope.
- `final-review` - fresh-context, multi-lens local review cycles before the
  repository's selected delivery action or a readiness claim, using local
  evidence when a mode has no pushed build and preserving every failed-run hold.
- `systematic-debugging` - compact root-cause debugging before fixes.
- `receiving-code-review` - technical evaluation of review feedback before
  implementing or pushing back, followed by additive repair unless a specific
  amend is explicitly authorized.
- `writing-skills` - concise skill authoring for this marketplace, with behavior
  fixtures where they are useful.

This plugin intentionally does not import upstream `using-superpowers`,
`brainstorming`, `subagent-driven-development`, `dispatching-parallel-agents`,
`using-git-worktrees`, or `finishing-a-development-branch`. Those workflows
conflict with or duplicate existing local practice.

## Capability boundary

The plugin is inert without a valid root `.development-system.toml`. Its
plugin-wide MCP surface is advisory: it exposes bounded repository inspection,
explicitly confirmed repository-local setup, and the multi-agent final-review
coordinator. It does not expose the native workflow lifecycle, inspect arbitrary shell
commands, tool names, patches, or Git arguments, execute project mutations, or
restrict ordinary harness tools. Editor, runner, local-repository,
remote-repository, and diagnostics services remain withheld for standalone
Tiber to authorize behind its own isolation boundary.

The advisory final-review coordinator nevertheless rejects zero-lens plans and
does not record completion until every selected lens and assigned verifier has
produced at least three consecutive complete finding-free iterations. Any
reported finding, malformed result, or material delta resets the streak, and a
review-budget `ship` decision cannot lower or bypass the requirement.
`workspace-reader.status` exposes a versioned final-review protocol attestation;
the installed workflow requires the current attestation before creating risk or
review state and rejects any plan that returns a lower clean-iteration minimum.
Schema-invalid lens or verifier evidence and verifier provenance/coverage
failures are normalized to bounded malformed-result records inside the
authoritative advance transition. The coordinator closes any pending verifier,
invalidates the whole iteration, and reissues every selected lens, so correcting
and retrying the submission cannot preserve an earlier clean streak.

## Harness

Codex consumes the canonical routing policy from `skills/`. The plugin packages
four task-local agents:
`bounded-helper`, `substantive-worker`, `strong-reviewer`, and `strong-worker`.
Codex agents pin
the exact GPT-5.6 model identifiers and sandbox modes. If Codex cannot honor the
requested route, the agent reports that failure
instead of treating inheritance or substitution as success.

The final-review coordinator runs from the Development System plugin's
versioned host-local binary installation. Build it with
`just install-development-system-binaries` from the marketplace
checkout after every plugin install or upgrade. The launcher never compiles
source at runtime.

The caller carries compact final-review state references between requests while
the MCP records typed transition facts on the Git-backed
`development-workflow` EventCore authority. Mutated, stale, or concurrently
superseded state and post-completion transitions fail closed with sanitized
recovery diagnostics; active sessions and retained review history are bounded.

The native checked-development lifecycle remains an internal service for
standalone Tiber because its RED, GREEN, verification, and delivery transitions
depend on assignment-bound mutation and runner receipts. The advisory plugin
does not advertise that incomplete lifecycle. Tiber is already the sole pushed-CI recovery authority on its
Git-backed EventCore store. Development Discipline reads its unresolved
incident hold when evaluating delivery readiness; matching terminal-success
replacement evidence in Tiber releases that hold.
