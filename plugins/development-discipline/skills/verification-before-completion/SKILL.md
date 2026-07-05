---
name: verification-before-completion
description: Use when claiming work is done, fixed, passing, ready, reviewed, committed, or safe to merge.
---

# Verification Before Completion

Evidence comes before claims. A completion statement is only valid when fresh
verification proves the exact scope being claimed.

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

## Red Flags

- "Should pass", "looks good", "probably fixed", or any success wording before
  running verification.
- Reusing a previous run after files changed.
- Treating a generated report, agent message, or green subcommand as proof of a
  broader claim.
- Skipping verification because the change is "just docs" or "obvious".

When evidence is missing, say what was checked and what remains unverified.
