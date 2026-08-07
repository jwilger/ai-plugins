---
name: test-driven-development
description: Use when implementing a feature, bugfix, production behavior change, removal, or refactor to decide whether RED applies and, when it does, work from one failing public behavior test at a time.
---

# Test-Driven Development

Require RED only for new or changed first-party production behavior without a
clear existing failing test. When RED applies, write a public black-box behavior
test first, watch it fail for the intended behavioral reason, write the smallest
implementation that makes it pass, and refactor only while green.

## Decide Whether RED Applies

Do not create a new failing test for:

- documentation, metadata, or another change that is not code;
- functionality removal;
- a test that would primarily restate documented third-party behavior;
- assertions that committed static text or file structure exists;
- a change already proved by a clear failing test;
- straightforward CI workflow scripting, where executing CI is the test;
- simple developer-environment setup or utilities with no production effect;
- behavior-preserving refactoring with adequate existing green black-box
  coverage.

If an existing failing test is correct but unclear, improve its diagnostic
message when possible without weakening the test. For third-party integrations,
explicitly reject tests that restate the vendor API. Test only the application's
observable product contract through a public, end-to-end black-box boundary;
the test should remain valid if the dependency is replaced. If that product
behavior is new and uncovered, write this application-level integration test
RED first. Verify exempt work with the relevant formatter, validator, unchanged
test suite, CI run, or direct utility execution. If a developer utility is too
complex for direct execution to establish confidence, extract it into a
maintained project or shipped subsystem and preserve coverage there.

For a removal, change production code first while leaving tests untouched. Run
the relevant suite and classify every failure. Delete or update only tests that
genuinely specify the removed behavior. Any failure covering behavior that must
remain means the implementation removed too much; repair it before returning
the full relevant suite to green. Never replace the old tests with a test that
only asserts the capability is absent.

## Rules

- One test at a time.
- One assertion or observable behavior per test.
- Prefer public, black-box behavior tests.
- Never open a committed repository file merely to assert expected text or
  structure. This includes documentation, fixtures, policies, skills,
  manifests, and configuration.
- Never test CI workflow definitions or job structure. Executing the workflow
  in CI is the test.
- RED must fail because the behavior is missing, not because of typos, compile
  errors, broken setup, or missing fixtures.
- GREEN is the smallest change that passes the current test.
- REFACTOR starts only after the relevant test and existing gate are green.
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
5. Run the focused test and the relevant existing checks.
6. Run a lightweight post-implementation review before the next testing cycle.
7. Refactor only with the tests green and the lightweight review clean.
8. Repeat for the next behavior.

## Green Increment Delivery

Treat each completed behavior as a preservable implementation increment. Before
starting the next RED test:

1. Run the fast unit tests and directly relevant quick checks.
2. Run the lightweight review below. If it causes an edit, repeat the fast tests
   and lightweight review until both are green.
3. Use `rationale-commit-messages`, then commit and push the green increment.
4. Check CI before starting a new task. The most recently **completed** build
   must be successful. A newer queued or running build does not replace that
   completed result, but a completed failed job in any current build invokes
   `ci-failure-follow-up`; its exact diagnosis, constrained next push, and
   terminal-success recovery rule blocks follow-up implementation,
   review-finding remediation, and a new ticket.

Long-running integration, mutation, exhaustive, full-suite, and similarly
expensive checks belong in CI unless a local run is directly required to
diagnose a failure. Do not make every local increment wait for them.

Full review is the ticket-completion gate after the actual acceptance criteria
are implemented; it is not a prerequisite for preserving each green increment.
When full review requires a code or guidance edit, first confirm that the most
recently completed pushed build is successful and no current build has a
completed failed job, classify whether RED applies and use it when
required, then repeat fast
unit tests, lightweight review, commit and push, and the CI check. Resume full
review through one diff-bound delta risk assessment rather than restarting
unaffected lenses.

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

| Signal                                                 | Action                                                  |
| ------------------------------------------------------ | ------------------------------------------------------- |
| Production code exists without prior RED when required | Revert or discard it, then restart from the test        |
| Test passes immediately                                | Replace it with a test for missing behavior             |
| Test checks internals or mocks instead of behavior     | Rewrite against the public surface                      |
| Test checks committed text or CI workflow structure    | Remove it or replace it with observable behavior        |
| Several cases are bundled into one test                | Split them unless this is the acceptance scenario table |
| You want to "add tests after"                          | Stop; that is not TDD                                   |
| Lightweight review is skipped after GREEN              | Run it before starting the next RED cycle               |

## Completion Check

When RED applies, point to the RED output, GREEN output, and the small
implementation step connecting them. For exempt work, point to the applicability
decision and fresh verification; for removals, also show the unchanged-suite
failure classification. Before starting the next cycle,
also point to the clean lightweight review or the defended finding accepted by a
follow-up review, the pushed commit, and a successful most-recently-completed
CI build (with no current completed failed job).
