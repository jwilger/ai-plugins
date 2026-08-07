---
name: systematic-debugging
description: Use when a bug, failing test, broken command, unexpected output, or confusing runtime behavior appears.
---

# Systematic Debugging

Establish a causal mechanism before changing production code. Keep the loop
compact: characterize, reproduce, hypothesize, discriminate, then fix.

Apply `model-routing` to each delegated debugging task. A bounded helper may
collect independently verifiable evidence, but ambiguous diagnosis and any
stronger responsibility use the route defined by that canonical matrix.

## Loop

1. Record expected versus observed behavior, exact error/output, command and
   inputs, exit code, revision, and environment facts that can affect the run.
2. Build the minimal reproducer: the smallest command or scenario that retains
   the same failure mechanism, not merely the same error text.
3. Inspect recent changes, boundary state, and a nearby working comparison.
4. State one falsifiable causal hypothesis: "X causes Y through mechanism Z; if
   true, observation A will differ from control B."
5. Run one discriminating experiment that observes A and B while holding
   unrelated variables constant. A reversible diagnostic change may collect
   evidence; it is not a production fix.
6. If the prediction holds, capture the reproducer as a failing regression test
   when RED applies, then change the earliest controllable cause in the failing
   path rather than masking the terminal symptom.
7. Verify that the original reproducer changes from failing to passing and run
   the owning component's required regression gate.

## Rules

- One hypothesis at a time.
- Do not bundle speculative fixes.
- Fix the source of the bad state, not the line where it finally explodes.
- If the failure spans components, add temporary evidence at the boundaries to
  find where the data or state changes.
- Count a failed fix attempt only when production code was mutated, tested
  against the reproducer, and rejected. Evidence-gathering experiments do not
  consume the count. After three failed fix attempts, stop and question the
  architecture or problem framing instead of adding a fourth mutation.

## Common Traps

| Trap                                          | Correction                                             |
| --------------------------------------------- | ------------------------------------------------------ |
| "This is obvious"                             | Verify the cause anyway                                |
| Changing several things before rerunning      | Isolate one variable                                   |
| Skipping the failure output                   | Read it first; it often names the issue                |
| Writing the regression test after the fix     | Capture the failure before the fix when practical      |
| Treating environmental failures as unknowable | Document what was ruled out and add useful diagnostics |
