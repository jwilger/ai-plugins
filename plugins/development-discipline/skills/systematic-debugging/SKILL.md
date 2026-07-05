---
name: systematic-debugging
description: Use when a bug, failing test, broken command, unexpected output, or confusing runtime behavior appears.
---

# Systematic Debugging

Find the root cause before changing code. Keep the loop compact: read,
reproduce, hypothesize, test, then fix.

## Loop

1. Read the exact error, stack trace, command, exit code, and recent output.
2. Reproduce the failure with the smallest reliable command or scenario.
3. Inspect recent changes and nearby working examples.
4. State one hypothesis: "I think X is causing Y because Z."
5. Test that hypothesis with the smallest observation or reversible change.
6. If confirmed, write a failing regression test when practical, then fix the
   root cause.
7. Verify the fix with the reproduction and the relevant broader gate.

## Rules

- One hypothesis at a time.
- Do not bundle speculative fixes.
- Fix the source of the bad state, not the line where it finally explodes.
- If the failure spans components, add temporary evidence at the boundaries to
  find where the data or state changes.
- After three failed fix attempts, stop and question the architecture or the
  framing instead of adding a fourth guess.

## Common Traps

| Trap                                          | Correction                                             |
| --------------------------------------------- | ------------------------------------------------------ |
| "This is obvious"                             | Verify the cause anyway                                |
| Changing several things before rerunning      | Isolate one variable                                   |
| Skipping the failure output                   | Read it first; it often names the issue                |
| Writing the regression test after the fix     | Capture the failure before the fix when practical      |
| Treating environmental failures as unknowable | Document what was ruled out and add useful diagnostics |
