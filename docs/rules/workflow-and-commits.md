# Workflow, commits, and delivery

- **One major change at a time.** Do not start another major task while a PR is
  still waiting on CI, review, approval, merge, or local cleanup.
- **PR-based** with **≥1 required approval** and **automated code review/approval**.
  CI gates: formatting, tests, marketplace validation, and Codex cross-harness
  manifest verification.
- **Conventional Commits.** Commit between BDD steps once `just ci` is green.
- **No `Co-Authored-By` trailers** (and no other AI-attribution trailers).
- **Forge-agnostic, no preference:** GitHub, Forgejo, and GitLab are first-class
  peers. Use the forge tooling that matches the repository remote.
- **Document every architectural decision** as an ADR in `docs/adr/` (why,
  alternatives considered, and the conditions under which to revisit).
