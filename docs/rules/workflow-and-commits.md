# Workflow, commits, and delivery

- **One major change at a time.** Do not start another major task while a PR is
  still waiting on CI, review, approval, merge, or local cleanup.
- **PR-based** with **≥1 required approval** and **automated code review/approval**;
  releases are managed (this repo: forgejo + release-plz → crates.io). CI gates:
  fmt, clippy `-D warnings`, tests, mutation (release-gated), audit, plus
  marketplace validation and codex cross-harness verification.
- **Manage Rust dependencies only via `cargo add`** so versions and feature flags
  are checked at the time of change — never hand-edit `[dependencies]`.
- **Conventional Commits.** Commit between BDD steps once `just ci` is green.
- **No `Co-Authored-By` trailers** (and no other AI-attribution trailers).
- **Forge-agnostic, no preference:** GitHub, Forgejo, and GitLab are first-class
  peers, auto-detected from the remote. `gh` does not work against Forgejo remotes
  (`git.johnwilger.com`) — prefer the Forgejo MCP tools, with `tea` as a fallback.
- **Document every architectural decision** as an ADR in `docs/adr/` (why,
  alternatives considered, and the conditions under which to revisit).
