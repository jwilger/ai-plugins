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
- `test-driven-development` - Kent Beck-style TDD: one failing behavior test,
  one smallest implementation step, then refactor only after green.
- `delivery-workflow` - routes delivery through repository-local instructions,
  after current user direction, supporting direct-to-trunk, PR/MR, and
  local-only work without inventing a pull request or letting a specialist skill
  change the selected mode, cadence, or evidence level; repairs default to new
  additive commits, and amends require explicit case-by-case authorization.
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

## Harnesses

Claude Code and Codex consume the same canonical routing policy from `skills/`.
The plugin also packages four task-local agents for each harness:
`bounded-helper`, `substantive-worker`, `strong-reviewer`, and `strong-worker`.
Codex agents pin
the exact GPT-5.6 model identifiers and sandbox modes. Claude agents use the
current Haiku, Sonnet, and Opus aliases with route-appropriate tool allowlists.
If a harness cannot honor the requested route, the agent reports that failure
instead of treating inheritance or substitution as success.

The final-review coordinator ships static stdio MCP binaries for x86_64 and
aarch64 Linux plus both macOS architectures. Its launcher selects the local
target without a runtime package installation; an explicitly enabled Cargo
fallback remains available for source-tree development. Release checks validate
each artifact's target format, checksum, and embedded source/toolchain
fingerprint.

The caller carries final-review state between requests, while one persistent MCP
process keeps the authoritative session copy. Mutated or stale state and
post-completion transitions fail closed; active sessions and retained review
history are bounded.
