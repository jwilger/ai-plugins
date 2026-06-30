# Testing — BDD, black-box, one step at a time

**Vertical slices, not layers.** Each unit of work delivers a user-observable
behavior end-to-end. Plans are shaped around behaviors, never component waterfalls.

**BDD / Cucumber** covers all externally-observable behavior, including edge cases.
Scenarios are **black-box**: they exercise the public surface only (the `sidequest`
binary via `assert_cmd`, and the MCP server via an MCP client) and **never touch
internal modules or private types**. Cross-harness behaviors use a Scenario Outline
with `Examples: codex, claude` so parity is part of each slice's definition of done.

**One Gherkin step at a time:** get a single Given/When/Then green with **all gates
passing** (`just ci`), **commit**, then move to the next step. Tests assert
behavior, never source-file text (no tautological "file contains string" tests).

**Mutation testing** (`cargo mutants`, `just mutants`): a **100% mutant kill rate**
is required; surviving mutants block release.
