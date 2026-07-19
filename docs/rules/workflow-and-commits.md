# Workflow, commits, and delivery

- **One major change per worktree at a time.** In PR mode, do not start another
  major task in that worktree while its PR is still waiting on CI, review,
  approval, merge, or local cleanup.
- **Trunk push CI by default; PR CI when explicitly configured.** Direct pushes
  to the default branch run the full gate. PR mode also uses merge-queue CI and
  requires **≥1 approval** plus **automated code review/approval**. CI gates:
  formatting, tests, marketplace validation, and Codex cross-harness manifest
  verification.
- **Rationale-bearing Conventional Commits.** Commit between BDD steps once
  `just ci` is green. Every authored commit needs a concise Conventional Commit
  subject and a non-empty body explaining why the change is necessary, such as
  the motivation, tradeoff, or failure it prevents. A subject-only message, or
  a body that merely repeats what changed, is not complete.
- **One-hour scope check.** As a rough heuristic, if no commit has been pushed
  in the past hour, pause and ask whether the current increment is being
  over-engineered. Prefer a smaller semantic increment when possible; the
  heuristic never permits skipping tests, review, or another required gate.
- **No `Co-Authored-By` trailers** (and no other AI-attribution trailers).
- **Forge-agnostic, no preference:** GitHub, Forgejo, and GitLab are first-class
  peers. Use the forge tooling that matches the repository remote.
- **Document every architectural decision** as an ADR in `docs/adr/` (why,
  alternatives considered, and the conditions under which to revisit).
