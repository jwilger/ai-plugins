# Workflow rules

## Change-impact preflight

Before editing, classify the change and decide each surface explicitly:
Behavior, Tests, Documentation, Configuration, Packaging, Release artifacts,
Migrations, Operational startup, Evaluations, and User workflows. For every
surface, record `applicable` or `not-applicable`, the impact relation
(`directly-modified`, `consumes`, `generates-from`, `validates`, `ships`, or
`none`), and repository evidence. File proximity is not an impact relation.

Provider-backed evaluation applies only when the change can alter
model-mediated behavior, such as instructions, triggers, model-visible schemas
or results, injected context, prompt construction, routing, fixtures, or
graders. A plugin-directory path is not sufficient evidence. For deterministic
installer, packaging, lifecycle, locking, path, state, permission, manifest, or
non-instructional documentation changes, run the relevant deterministic tests
and eval-wiring dry run, then explicitly record live behavior evals as not
applicable. When live evaluation is applicable, name the changed claim and
metric, then start with the smallest causal case set and an explicit sample
count justified by that metric; expand to a full-marketplace suite only when a
shared model-facing surface or marketplace loading is the claim. Use only the
live harnesses selected by current repository policy.

## CI recovery record

For a terminal pushed-CI failure, retain four records:

- Failure: revision, run ID or URL, failed job and step, and relevant log evidence.
- Diagnosis: causal explanation plus `caused`, `unrelated`, or `transient`, with evidence.
- Next action: one tested causal repair, or an authorized unchanged-revision rerun; never a no-op or intervening diagnostic commit.
- Release proof: replacement run ID, exact revision, and terminal success. Queued,
  pending, or running never releases the hold.

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
- Debug from a minimal reproducer, one falsifiable causal hypothesis, its
  predicted observation, and a discriminating experiment. Repair the earliest
  controllable cause supported by that evidence.
- Final review is required before a readiness claim.
- Final review is clean only after at least three consecutive complete
  finding-free iterations. Every risk-selected lens and required verifier must
  participate in each iteration; any finding or material delta resets the
  streak, and neither a review-budget decision nor caller-carried state may
  bypass it.
- Verification evidence is fresh only when collected after the last in-scope
  mutation against the exact revision or worktree snapshot and current
  configuration. It is scope-complete only when it includes repository-required
  gates plus impact-selected checks. Map each readiness claim to its command or
  authoritative source, revision, environment identity, timestamp, and result.
- Every authored commit has a concise Conventional Commit subject and a
  non-empty body that explains why the change exists: its motivation, decision
  context, tradeoff, or the failure it prevents. Reject subject-only messages
  and bodies that merely restate the subject or diff.
- Never add `Co-Authored-By` trailers.
- A terminal pushed-CI failure creates a repository-wide hold. Tiber records
  the exact failure, ownership lease, diagnosis, either one causal repair or an
  unchanged-SHA rerun, and terminal-success proof. Development Discipline reads
  that unresolved Tiber hold when authorizing delivery; it never stores a
  parallel incident. Projects that need CI-recovery enforcement therefore
  enable Tiber rather than silently falling back to a second workflow store.
