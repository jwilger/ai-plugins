# Testing — BDD, black-box, one step at a time

**Vertical slices, not layers.** Each unit of work delivers a user-observable
behavior end-to-end. Plans are shaped around behaviors, never component waterfalls.

**Behavior tests** cover externally-observable behavior, including edge cases.
Scenarios are **black-box**: they exercise the public surface and avoid private
implementation details. Cross-harness behaviors should cover Claude Code and
Codex when both harnesses are affected.

**One behavior step at a time:** get one observable behavior green with **all
gates passing** (`just ci`), **commit**, then move to the next step. Tests assert
behavior, never source-file text (no tautological "file contains string" tests).
