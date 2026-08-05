# Testing — BDD, black-box, one step at a time

**Vertical slices, not layers.** Each unit of work delivers a user-observable
behavior end-to-end. Plans are shaped around behaviors, never component waterfalls.

**Behavior tests** cover externally-observable behavior, including edge cases.
Scenarios are **black-box**: they exercise the public surface and avoid private
implementation details. Cross-harness behaviors should cover Claude Code and
Codex when both harnesses are affected.

**One behavior step at a time:** get one observable behavior green with **all
gates passing** (`just ci`), **commit**, then move to the next step.

Start with RED only when adding or changing first-party production behavior and
no existing failing test already proves the required change. Do not invent a
new failing test for:

- documentation, metadata, or another non-code-only change;
- functionality removal;
- third-party behavior that is already documented and is not our contract;
- committed-file text or structure;
- a change already identified by a clear failing test;
- straightforward CI workflow scripting, where executing CI is the test;
- simple developer-environment setup or utilities that do not affect production;
- a behavior-preserving refactor with adequate green black-box coverage.

If an existing failure is correct but unclear, improve its diagnostic message
when that can be done without weakening the test. Test integrations through the
application's public, end-to-end behavior so coverage can survive replacement of
the third-party implementation. If a developer utility needs extensive tests to
be trustworthy, extract it into an independently maintainable project or shipped
subsystem instead of growing a private utility test architecture indefinitely.

For removal work, remove the functionality before changing its tests. Run the
unchanged relevant suite, then classify every failure. Delete or update only
tests that genuinely specify the removed behavior. A failure covering behavior
that must remain is evidence that the implementation removed too much; repair
the implementation before returning the complete relevant suite to green. Do
not add a replacement test whose only assertion is that the capability is gone.

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

Reviewers apply the same applicability rules before reporting missing coverage
or missing RED evidence. When they encounter an existing test that violates
these rules, they report it and recommend removal, replacement with public
black-box behavior, or extraction of an overgrown developer utility. They do not
strip important coverage from a complex utility before its replacement home is
ready.
