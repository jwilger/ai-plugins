---
name: verification-before-completion
description: Use when claiming work is done, fixed, passing, ready, reviewed, committed, or safe to merge.
---

# Verification Before Completion

Evidence comes before claims. A completion statement is only valid when fresh
verification proves the exact scope being claimed.

Apply `model-routing` to delegated evidence gathering and review. A helper may
collect bounded evidence, but the accountable completion or readiness decision
must remain on the strong route defined by that canonical matrix.

## Gate

1. State the claim precisely.
2. Identify the evidence that would prove that claim.
3. Run the relevant command or inspect the authoritative source now.
4. Read the full output or current state, including exit code and failures.
5. Report only what the evidence proves. Name any gaps.

## Claim Scope

| Claim                  | Required evidence                                                                  |
| ---------------------- | ---------------------------------------------------------------------------------- |
| "Tests pass"           | Fresh full relevant test output with zero failures                                 |
| "Lint/format is clean" | Fresh lint/format command output                                                   |
| "Bug fixed"            | The original symptom or regression test now passes                                 |
| "Requirement met"      | Requirement-by-requirement check against files, output, runtime state, or PR state |
| "Ready to merge"       | Current checks, review state, branch status, and required approvals                |

Partial checks prove partial claims only. A focused test can prove a narrow fix;
it cannot prove the whole repo is green.

## Bound Long-Running Verification

Before starting a long test, evaluation, CI build, or external check, record an
explicit timeout or monitoring and cancellation plan. A pending or running check
is waiting: it is neither blocked nor passing evidence.

For CI, use a comparable recent successful run as the duration baseline. When
the current run exceeds that baseline by roughly five unexplained minutes,
inspect the active step and logs before deciding whether it is hung. Extra work
shown in the logs is a reason to keep waiting; elapsed time alone is not a
reason to cancel.

Cancellation and retry require applicable authority and a recorded reason. A
timeout, hang, cancellation, or incomplete retry is never passing evidence.
Retain this record under a stable blocker reference:

```text
Verification record: <command or check>; elapsed time=<duration>;
  active step or last output=<evidence>; retained artifacts=<paths or URLs>
Blocker: <stable blocker reference>; monitoring or timeout=<plan>;
  cancellation or retry=<decision, authority, and reason>
Claim limit: <missing evidence and how it narrows the completion or readiness claim>
```

Track broken verification infrastructure separately and carry the same blocker
reference into later reviews. Do not rediscover the same failure each cycle or
replace the missing gate with fallback evidence. Fallback evidence may narrow
the completion or readiness claim; it cannot prove a claim that requires the
unfinished check.

## Red Flags

- "Should pass", "looks good", "probably fixed", or any success wording before
  running verification.
- Reusing a previous run after files changed.
- Treating a generated report, agent message, or green subcommand as proof of a
  broader claim.
- Skipping verification because the change is "just docs" or "obvious".

When evidence is missing, say what was checked and what remains unverified.
