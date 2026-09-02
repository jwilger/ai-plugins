---
name: test-driven-development
description: Use when implementing a feature, bugfix, production behavior change, removal, or refactor to decide whether RED applies and, when it does, work from one failing public behavior test at a time.
---

# Test-Driven Development

Require RED only for new or changed first-party production behavior without a
clear existing failing test. When RED applies, write a test first through a
stable observable boundary, watch it fail for the intended behavioral reason,
write the smallest implementation that makes it pass, and refactor only while
green.

A stable observable boundary is a supported interface whose result survives an
internal rewrite: for example CLI arguments plus exit status/stdout/files, an
HTTP request and response, a public API result, or an emitted event. A test is
not black-box merely because it runs outside the production module; it must not
read private symbols, source files, committed repository text, or workflow
structure to infer behavior.

## Decide Whether RED Applies

Do not create a new failing test for:

- documentation, metadata, or another change that is not code;
- functionality removal;
- a test that would primarily restate documented third-party behavior;
- assertions that committed static text or file structure exists;
- a change already proved by a clear failing test;
- straightforward CI workflow scripting, where executing CI is the test;
- a non-shipped, stateless developer-environment helper with no concurrency,
  persistence, destructive I/O, recovery contract, or production effect;
- behavior-preserving refactoring whose existing green stable-boundary tests
  would fail if any preserved externally observable invariant regressed.

If an existing failing test is correct but unclear, improve its diagnostic
message when possible without weakening the test. For third-party integrations,
explicitly reject tests that restate the vendor API. Test only the application's
observable product contract through a public, end-to-end black-box boundary;
the test should remain valid if the dependency is replaced. If that product
behavior is new and uncovered, write this application-level integration test
RED first. Verify exempt work with the owning formatter or validator, unchanged
component suite, repository-required gate, CI run, or direct utility execution.
A developer utility with concurrency, persistence, destructive I/O, or recovery
semantics is not a simple-helper exemption; extract it into a maintained project
or shipped subsystem and preserve coverage there.

For a removal, change production code first while leaving tests untouched. Run
the relevant suite and classify every failure. Delete or update only tests that
genuinely specify the removed behavior. Any failure covering behavior that must
remain means the implementation removed too much; repair it before returning
the full relevant suite to green. Never replace the old tests with a test that
only asserts the capability is absent.

## Rules

- One test at a time.
- One contract and failure reason per test. Multiple assertions may observe one
  contract outcome.
- Prefer tests through a stable observable boundary.
- Never open a committed repository file merely to assert expected text or
  structure. This includes documentation, fixtures, policies, skills,
  manifests, and configuration.
- Never test CI workflow definitions or job structure. Executing the workflow
  in CI is the test.
- RED must fail because the behavior is missing, not because of typos, compile
  errors, broken setup, or missing fixtures.
- GREEN is the smallest change that passes the current test without weakening
  another preserved contract.
- REFACTOR starts only after the focused test, owning component suite, and
  repository-required fast gate are green.
- No production code before the failing test has been observed when RED applies.

When a program creates or edits a file, first test the behavioral effect visible
to an end user. Assert exact generated text only when no behavioral-effect test
can prove the requirement, and only against output produced by the program
under test—not a pre-existing committed file. If there is no meaningful product
behavior to exercise, do not invent a test for coverage.

Apply this rule in every project. Whenever you encounter an existing test that
violates the applicability rules above, remove it, replace it with public
black-box behavior coverage, or preserve it until an overgrown utility is
extracted. Report the violation during review even when it predates the current
change.

For every proposed test, state the observable product behavior it proves. If a
file is involved, say whether it is pre-existing repository content or output
created by the program, and why a behavioral-effect assertion is or is not
possible. During implementation and review, inspect the surrounding test scope
for the same anti-pattern and remove or replace any instances you find.

Gherkin or acceptance specs may define a coherent scenario set up front. Even
then, implementation proceeds one step or scenario at a time.

## Loop

1. Classify whether RED applies and record the reason when it does not.
2. When RED applies, name the behavior in concrete user-observable terms and
   write the smallest failing test for it.
3. Run it and read the failure. If it fails for setup or syntax, fix the test
   until RED proves the behavior is missing.
4. Implement only enough code for that test. For removal work, instead follow
   the remove-first sequence above with the old tests unchanged.
5. Run the focused test and the owning component suite.
6. Run a lightweight post-implementation review, then the
   repository-required fast gate, before the next testing cycle.
7. Refactor only after the current GREEN snapshot has completed its fast gate,
   signed commit or authorized local-only equivalent, and delivery checkpoint;
   the refactor begins the next per-edit cycle.
8. Repeat for the next behavior.

## Green Increment Delivery

Treat every individual implementation-file or test-file edit as an interruptible
increment boundary. Immediately run the smallest relevant test and durably
record the exact snapshot as `failing` or
`passing-awaiting-gates-or-review`.

An immediately passing newly written test is not GREEN and must not enter
`passing-awaiting-gates-or-review`. Record the snapshot in canonical `failing`
state with `failure_kind: invalid-test` and the causal test rewrite as its sole
next action, then rerun it immediately. Commit and push remain prohibited until
a valid RED result (or an explicitly recorded RED exemption) is followed by
GREEN. `failure_kind` explains the failure without introducing another durable
checkpoint state.

At `failing`, prohibit commit, push, unrelated edits, cleanup, and convenience
changes. Permit only the next causal edit, then run the smallest relevant test
again immediately. When test and implementation cannot independently pass,
declare one bounded RED-to-GREEN pair: record the expected RED and causal claim,
make only the paired implementation edit, retest immediately, and continue from
GREEN.

At `passing-awaiting-gates-or-review`, stop all implementation and test editing:

1. Run the lightweight review below. Any remediation is a causal edit and
   returns immediately to the focused-test boundary.
2. Let the repository fast pre-commit gate run its formatting, linting, unit,
   manifest/config, and comparable fast checks. Do not duplicate comprehensive
   suites in a second local exact-commit gate.
3. When the selected mode authorizes or requires a commit, use
   `rationale-commit-messages`, create a signed commit, and record its exact OID
   and the snapshot transition to `committed`. In local-only mode without commit
   authority, record the exact reviewed and fast-gate-passing worktree snapshot
   as the authorized no-commit terminal equivalent instead, unless Tiber's
   opt-in final-review policy makes a local commit repository-required for the
   selected source and verification paths. Under that policy, withheld commit
   authority blocks completion without authorizing a push.
4. Complete the authorized delivery-mode checkpoint immediately and record
   `pushed-or-delivery-mode-equivalent`. Direct-to-trunk pushes normally; PR
   mode pushes only the already-authorized branch and does not infer permission
   to open or merge; local-only records the locally permitted terminal snapshot
   and performs no remote mutation.

Every interruption preserves the exact snapshot, immediate-test receipt,
durable state, completed gates, and sole next permitted action. Documentation,
configuration, formatting-only changes, generated output, and mechanically
required metadata use proportional formatter/parser/validation gates and the
same checkpoint discipline. Generated or required companions may travel only
with their causal source snapshot; unrelated passing work is never batched for
convenience.

After an authorized push, begin the next iteration immediately while monitoring
the exact pushed OID's CI concurrently. A queued or running build is not green,
but does not serialize feature work when the most recently completed in-scope
build is successful and no current required run has a completed failed job. Any
completed CI failure immediately preempts the current edit and invokes
`ci-failure-follow-up`; its recovery hold permits only the selected causal repair
or authorized unchanged-SHA rerun until terminal success.

Keep checkpoint CI bounded per ref. Use the repository or provider's existing
superseded-run cancellation or coalescing policy when available; otherwise,
before another checkpoint push would create an additional obsolete queued run,
wait for capacity or cancel only the superseded non-terminal run when that
mutation is already authorized. Never cancel the current terminal-review
candidate's exact-SHA run, and never let an older run satisfy its readiness
gate.

Long-running integration, mutation, exhaustive, full-suite, and similarly
expensive checks belong in CI unless a local run is directly required to
diagnose a failure. Do not make every local increment wait for them.

Full review is the ticket-completion gate after all planned increments and the
actual acceptance criteria are delivered; it is not a prerequisite for
preserving each green increment, and lightweight review never substitutes for
its configured clean iterations.
When full review requires a code or guidance edit, first confirm that the most
recently completed pushed build is successful and no current build has a
completed failed job, classify whether RED applies and use it when
required, then repeat the immediate focused test, lightweight review, fast
pre-commit gate, signed commit, exact verification, and authorized push. Only
then resume full review through its diff-bound delta/reset protocol.

## Lightweight Review

After each implementation step, run one fresh-context review subagent before
moving to the next RED test or scenario. This is a compact version of
`final-review`:

Apply `model-routing` to the review assignment. Ordinary lightweight review is
substantive work; activated ambiguity, security, human-safety, disputed
verification, or readiness responsibility must use the stronger route defined
there.

- combine the repository-agnostic final-review lenses into one reviewer prompt;
- include production-risk and footgun checks, especially data-access or
  resource-use patterns that pass lower environments but fail under production
  scale or burst load;
- ask for findings against the just-completed implementation step and its tests;
- treat committed-text assertions and CI-workflow-definition tests as
  actionable test-quality findings;
- apply the complete RED-applicability rules above before demanding coverage,
  and report any existing violation found in the surrounding test scope;
- require removal, observable-behavior replacement, or extraction as
  appropriate without deleting complex utility coverage prematurely;
- fix valid findings, or record a concise defense when not changing the code;
- continue only after one clean review, or after the next review accepts the
  defense.

For review-only or no-subagent environments, state that the lightweight review
cannot be completed to this standard instead of silently skipping it.

## Stop Signals

| Signal                                                        | Action                                                                    |
| ------------------------------------------------------------- | ------------------------------------------------------------------------- |
| Production code exists without prior RED when required        | Revert or discard it, then restart from the test                          |
| Test passes immediately                                       | Replace it with a test for missing behavior                               |
| Test checks internals or mocks instead of behavior            | Rewrite against the public surface                                        |
| Test checks committed text or CI workflow structure           | Remove it or replace it with observable behavior                          |
| Several cases are bundled into one test                       | Split them unless this is the acceptance scenario table                   |
| You want to "add tests after"                                 | Stop; that is not TDD                                                     |
| Lightweight review is skipped after GREEN                     | Run it before starting the next RED cycle                                 |
| Another implementation/test edit follows uncheckpointed GREEN | Stop and finish review, fast gate, signed commit, and delivery checkpoint |
| Unrelated passing work is added to the checkpoint             | Remove it and preserve only the causal snapshot                           |

## Completion Check

When RED applies, point to the RED output, GREEN output, and the small
implementation step connecting them. For exempt work, point to the applicability
decision and fresh verification; for removals, also show the unchanged-suite
failure classification. Before starting the next cycle, also point to the exact
durable state and next action, the clean lightweight review or defended finding
accepted by a follow-up review, and the signed commit plus authorized push or
the exact authorized local-only no-commit equivalent. Track exact-SHA CI
concurrently; a completed failure
preempts the next cycle.
