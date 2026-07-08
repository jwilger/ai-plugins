---
name: test-driven-development
description: Use when implementing any feature, bugfix, behavior change, or refactor before writing production code.
---

# Test-Driven Development

Write the test first. Watch it fail for the intended behavioral reason. Write
the smallest implementation that makes it pass. Refactor only while green.

## Rules

- One test at a time.
- One assertion or observable behavior per test.
- Prefer public, black-box behavior tests.
- RED must fail because the behavior is missing, not because of typos, compile
  errors, broken setup, or missing fixtures.
- GREEN is the smallest change that passes the current test.
- REFACTOR starts only after the relevant test and existing gate are green.
- No production code before the failing test has been observed.

Gherkin or acceptance specs may define a coherent scenario set up front. Even
then, implementation proceeds one step or scenario at a time.

## Loop

1. Name the behavior in concrete user-observable terms.
2. Write the smallest failing test for that behavior.
3. Run it and read the failure. If it fails for setup or syntax, fix the test
   until RED proves the behavior is missing.
4. Implement only enough code for that test.
5. Run the focused test and the relevant existing checks.
6. Run a lightweight post-implementation review before the next testing cycle.
7. Refactor only with the tests green and the lightweight review clean.
8. Repeat for the next behavior.

## Lightweight Review

After each implementation step, run one fresh-context review subagent before
moving to the next RED test or scenario. This is a compact version of
`final-review`:

- combine the repository-agnostic final-review lenses into one reviewer prompt;
- include production-risk and footgun checks, especially data-access or
  resource-use patterns that pass lower environments but fail under production
  scale or burst load;
- ask for findings against the just-completed implementation step and its tests;
- fix valid findings, or record a concise defense when not changing the code;
- continue only after one clean review, or after the next review accepts the
  defense.

For review-only or no-subagent environments, state that the lightweight review
cannot be completed to this standard instead of silently skipping it.

## Stop Signals

| Signal                                             | Action                                                  |
| -------------------------------------------------- | ------------------------------------------------------- |
| Production code exists without a prior RED test    | Revert or discard it, then restart from the test        |
| Test passes immediately                            | Replace it with a test for missing behavior             |
| Test checks internals or mocks instead of behavior | Rewrite against the public surface                      |
| Several cases are bundled into one test            | Split them unless this is the acceptance scenario table |
| You want to "add tests after"                      | Stop; that is not TDD                                   |
| Lightweight review is skipped after GREEN          | Run it before starting the next RED cycle               |

## Completion Check

Before moving on, be able to point to the RED output, the GREEN output, and the
small implementation step that connects them. Before starting the next cycle,
also point to the clean lightweight review or the defended finding accepted by a
follow-up review.
