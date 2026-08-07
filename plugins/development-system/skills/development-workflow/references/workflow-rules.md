# Workflow rules

- Preserve user changes and avoid destructive Git operations.
- New or changed first-party production behavior starts with a failing test
  unless a clear existing failure already proves the change. Documentation and
  other non-code work, functionality removal, third-party behavior, committed
  static text, straightforward CI scripting, simple non-production developer
  utilities, and behavior-preserving refactors with adequate green coverage do
  not require a new RED test.
- Remove functionality before changing its tests. Run the unchanged suite,
  delete or update only expectations that genuinely describe the removed
  behavior, and repair collateral regressions in behavior that must remain. A
  failure in shared or retained behavior proves the implementation removed too
  much. Never add a replacement test whose sole assertion is that the removed
  capability is absent.
- Reviewers use the same applicability decision and audit the surrounding suite,
  including violations that predate the diff. Report committed README/static
  text assertions and CI-workflow-structure assertions for removal, because
  formatting and actual CI execution are the evidence. Report tests that merely
  mirror documented vendor examples for removal or replacement with
  dependency-agnostic black-box coverage of an application integration
  contract. Retain user-visible black-box coverage. If a developer-only utility
  has grown complex enough to need extensive locking, crash-recovery, or similar
  tests, recommend extracting it into a maintained project or shipped subsystem
  and preserve its valuable coverage until the extraction carries it forward.
- Debug from observed evidence and repair the causal defect.
- Final review is required before a readiness claim.
- Verification output must be fresh and relevant to the changed surfaces.
- Every authored commit has a concise Conventional Commit subject and a
  non-empty body that explains why the change exists: its motivation, decision
  context, tradeoff, or the failure it prevents. Reject subject-only messages
  and bodies that merely restate the subject or diff.
- Never add `Co-Authored-By` trailers.
- A terminal pushed-CI failure creates a repository-wide hold. The Development
  Discipline workflow coordinator records the exact failure, chooses either one
  causal repair or an unchanged-SHA rerun, and releases the hold only after
  terminal-success proof. This coordination is independent of Tiber and works
  in repositories that have no task board.
