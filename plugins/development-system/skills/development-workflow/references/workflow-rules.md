# Workflow rules

- Preserve user changes and avoid destructive Git operations.
- A runtime behavior change starts with a failing executable acceptance test.
  Prefer Gherkin when the project supports it, then implement scenario steps one
  at a time and add focused unit tests whenever the current failure is not one
  tightly scoped semantic unit. Documentation-only work does not invent tests;
  CI workflow execution is the behavioral test for CI-only changes.
- Debug from observed evidence and repair the causal defect.
- Final review is required before a readiness claim.
- Verification output must be fresh and relevant to the changed surfaces.
- Commit messages explain why the change exists and use Conventional Commits.
- After terminal delivery from a clean linked worktree, use the semantic worktree finish tool to return to primary and remove the worktree without deleting its branch.
- Never add `Co-Authored-By` trailers.
- An unexpected terminal pushed-CI failure creates a repository-wide Beads
  hold. One atomically claimed `ci-recovery` molecule and the project merge slot
  identify the owner. Release both only after exact terminal-success proof. An
  intentional failure inside a `ci-workflow-slice` is related test work.
