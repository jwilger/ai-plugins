# Workflow rules

- Preserve user changes and avoid destructive Git operations.
- A behavior change starts with a failing test unless the user requests a
  disposable prototype.
- Debug from observed evidence and repair the causal defect.
- Final review is required before a readiness claim.
- Verification output must be fresh and relevant to the changed surfaces.
- Commit messages explain why the change exists and use Conventional Commits.
- Never add `Co-Authored-By` trailers.
- A terminal pushed-CI failure creates a repository-wide hold. One Tiber owner
  records the exact failure, chooses either one causal repair or an unchanged-SHA
  rerun, and releases the hold only after terminal-success proof.
