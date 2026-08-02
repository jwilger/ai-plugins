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
- Use provider evals only for a named stochastic question that deterministic
  verification cannot decide. Deterministic retirement, deletion, file,
  routing, schema, format, metadata, and transformation contracts—and skill
  prose edits that change only such contracts—need no live eval. Fail closed on
  uncertain baseline/treatment isolation. Lift gates require an explicit
  incremental-value hypothesis; unexpected baseline success requires an
  isolation/context/prompt/rubric audit and deliberate retirement or
  absolute-reliability disposition, never a blind `valueGate.mode: none`.
- Commit messages explain why the change exists and use Conventional Commits.
- Create a linked worktree only when concurrent mutable tickets need isolation. After terminal delivery, cleanup of an unused clean worktree is optional housekeeping; run repository teardown before removal and never force it.
- Never add `Co-Authored-By` trailers.
- In direct-to-trunk mode, a verified semantic checkpoint may be pushed when
  the most recent completed run for the configured trunk branch that reached a
  pass/fail outcome passed and no unresolved failure hold exists. Newer queued,
  pending, or running runs do not replace that watermark. A canceled run is
  non-evidence: it neither passes nor fails, does not replace the watermark, and
  does not create or release a hold. Ticket completion is a separate gate and
  requires terminal success for the exact final pushed SHA.
- An unexpected terminal pushed-CI failure creates a repository-wide Beads
  hold immediately. One atomically claimed `ci-recovery` molecule and the
  project merge slot identify the owner. Stop unrelated work and permit only
  causal recovery until terminal success of either the exact tested
  causal-repair revision or the authorized rerun of the exact unchanged failed
  SHA. An intentional failure inside a `ci-workflow-slice` is related test work.
