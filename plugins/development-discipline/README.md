# development-discipline

John's personal workflow plugin for development discipline. It packages the
workflow skills that should replace the upstream `superpowers` variants in day
to day work, tuned for this marketplace and personal reuse rather than public
generality.

## Skills

- `test-driven-development` - Kent Beck-style TDD: one failing behavior test,
  one smallest implementation step, then refactor only after green.
- `verification-before-completion` - evidence-before-claims discipline tied to
  the actual claim scope.
- `systematic-debugging` - compact root-cause debugging before fixes.
- `receiving-code-review` - technical evaluation of review feedback before
  implementing or pushing back.
- `writing-skills` - concise skill authoring for this marketplace, with behavior
  fixtures where they are useful.

This plugin intentionally does not import upstream `using-superpowers`,
`brainstorming`, `subagent-driven-development`, `dispatching-parallel-agents`,
`using-git-worktrees`, or `finishing-a-development-branch`. Those workflows
conflict with or duplicate existing local practice.

## Harnesses

Harness-agnostic. Claude Code and Codex both consume the same `skills/`
contents, with separate marketplace manifests only for harness metadata.
