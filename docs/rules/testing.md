# Testing — BDD, black-box, one step at a time

**Vertical slices, not layers.** Each unit of work delivers a user-observable
behavior end-to-end. Plans are shaped around behaviors, never component waterfalls.

**Behavior tests** cover externally-observable behavior, including edge cases.
Scenarios are **black-box**: they exercise the public surface and avoid private
implementation details. Cross-harness behaviors should cover Claude Code and
Codex when both harnesses are affected.

**One behavior step at a time:** get one observable behavior green with **all
gates passing** (`just ci`), **commit**, then move to the next step.

Tests assert application or library behavior, never facts copied from committed
repository files. Do not add tests that open committed documentation, fixtures,
policies, skills, manifests, or configuration and check for expected text or
structure. Do not test CI workflow definitions or job structure; executing the
workflow in CI is the test.

When the product creates or edits a file, prefer the behavioral effect visible
to an end user. Assert exact generated text only when no behavioral-effect test
can prove the requirement, and only against output produced by the program under
test—not a pre-existing committed file. When an existing test violates these
rules, remove it or replace it with a public black-box behavior test. If no
meaningful product behavior exists, do not invent a test for coverage.
