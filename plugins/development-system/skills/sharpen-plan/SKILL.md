---
name: sharpen-plan
description: Use when a plan needs one highest-leverage assumption or specification clarified before approval, then repeat approved passes only until further refinement would be implementation detail or low-value polish.
---

# Sharpen plan

Apply exactly one highest-leverage assumption or plan-level specification per
pass to make the plan more likely to succeed.

## Run one pass

1. Read the complete current plan and identify exactly one missing assumption or
   plan-level specification whose resolution most reduces failure or rework.
   Prefer an upstream contract, boundary, invariant, sequence constraint,
   acceptance condition, or operating-environment constraint that is not
   already covered.
2. Keep the improvement at plan altitude. Do not add function signatures,
   field inventories, file-by-file edits, tuning constants, command details, or
   other implementation choices that can be settled safely during execution.
3. Ask no question unless the improvement exposes a genuine fork that the user
   must decide. For such a fork, ask exactly one question through the
   harness-native question mechanism, recommend an option when the evidence
   supports one, and wait for the answer. Do not bundle follow-up questions.
4. Apply the single improvement to the active plan artifact or conversational
   plan state. If Plan Mode or another read-only planning context forbids edits,
   preserve that boundary and update only the conversation; do not write a plan
   file.
5. Re-present the complete updated plan, not merely the changed section,
   through the harness-native approval mechanism. End the pass and wait for
   approval. Do not begin implementation or another sharpening pass.
6. Repeat only after approval, starting from the updated plan. A rejection or
   requested revision is feedback on the current pass, not approval to continue
   the loop.

When already running as `sharpen-plan`, perform this judgment directly. Do not
invoke Advisor or another `sharpen-plan` agent recursively. Keep the loop
approval-driven and self-contained; do not create or depend on Stop hooks,
session goals, or companion plan and review-plan machinery.

## Stop at diminishing returns

If no load-bearing assumption or plan-level contract remains, say plainly that
sharpening has reached diminishing returns and the plan is ready for execution.
Stop instead of inventing a cosmetic edit, restating existing content, or
promoting implementation minutiae into the plan.
