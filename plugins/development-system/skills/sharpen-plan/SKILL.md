---
name: sharpen-plan
description: Use when a plan needs one highest-leverage assumption or specification clarified before approval, then repeat approved passes only until further refinement would be implementation detail or low-value polish.
---

# Sharpen plan

Have a confirmed highest-capability read-only author apply exactly one
highest-leverage assumption or plan-level specification per pass. Keep the
parent accountable for scope, authority, user questions, and approval.

## Dispatch the sharpening pass

1. If already running as the assigned sharpening author, perform the judgment
   and revised-plan authorship directly. Do not invoke Advisor, `sharpen-plan`,
   or another strong agent recursively.
2. Inspect the eligible models advertised by the current harness and its
   authoritative capability or upgrade metadata. Explicitly select the
   highest-capability eligible model for a read-only assignment. Never infer
   capability from a model name, lexical or list order, price, or release date,
   and do not pin a version-specific model.
3. Dispatch by harness with the explicit model selection and high effort:
   - On Claude Code, launch the exact named public read-only `strong-reviewer`
     Agent in the foreground, explicitly setting `run_in_background=false` when
     that control exists.
   - On Codex, inspect the exposed `spawn_agent` contract before invoking it.
     Proceed only if it accepts explicit `model` and `reasoning_effort` inputs
     and exposes a reliable way to confirm their effective values. Otherwise
     return exactly `Blocked: development-system:sharpen-plan cannot verify required Codex spawn_agent evidence fields: <comma-separated missing fields in task_name, model, reasoning_effort, fork_turns order>. No artifact was produced.` immediately: do not invoke
     `spawn_agent`, wait or poll, ask the sharpening question, or author or
     revise any plan content in the parent. When supported, call the generic
     `spawn_agent` mechanism with `fork_turns: "none"` or the smallest bounded
     turn count that carries necessary context, the explicitly selected
     highest-capability model, and high reasoning effort. After launch, confirm
     that exact model and effort before any wait or poll; if confirmation fails,
     stop without waiting or substituting parent-authored plan content. Put the
     complete sharpening-author role instructions and task context in the spawn
     message. Treat `task_name` only as an operational label, not as a role
     selector or evidence that the role contract was loaded.

   For either harness, confirm the selected model, high effort, effective
   read-only child sandbox, and non-background dispatch before relying on the
   result. Instructions alone are not a read-only sandbox. Wait for the child to
   complete and expose its substantive result to the parent; an empty wait,
   agent identifier, async or background launch metadata, pending status, or
   parent-authored substitute is not a sharpening result. Model choice supplies
   neither write authority nor approval to implement the plan.

4. Give the author the complete current plan, the user's goal, relevant source
   facts, constraints, prior answers, and the contract below. Ask it to choose
   exactly one highest-leverage clarification and author the complete revised
   plan, not merely a patch or commentary. Supply the active plan artifact or
   conversational plan state without converting one into the other.
5. If authoritative ranking, explicit selection, high-effort configuration,
   the effective read-only child sandbox, foreground launch, or a completed
   substantive result cannot be confirmed, name the failed step and return a
   visible bounded blocked result. Treat a background launch or missing
   completed result as a route failure even if the harness returned successful
   launch metadata. Stop the sharpening pass. Do not substitute a weaker or
   default route, make the judgment in the parent, present pending output, or
   silently reuse unconfirmed output.

The assignment remains read-only even outside Plan Mode: it may inspect the
plan and relevant evidence and return plan text, but it may not edit files,
commit, install software, mutate services, or begin implementation. If an
authorized plan artifact must be updated outside Plan Mode, the parent may
place the strong author's returned plan into that artifact without changing
its substance. No other write or scope expansion follows from the assignment.

## Author one pass

When running as the assigned sharpening author:

1. Read the complete current plan and identify exactly one missing assumption or
   plan-level specification whose resolution most reduces failure or rework.
   Prefer an upstream contract, boundary, invariant, sequence constraint,
   acceptance condition, or operating-environment constraint that is not
   already covered.
2. Keep the improvement at plan altitude. Do not add function signatures,
   field inventories, file-by-file edits, tuning constants, command details, or
   other implementation choices that can be settled safely during execution.
3. Ask for no user input unless the improvement exposes a genuine fork that the
   user must decide. When it does, return exactly one concise question and a
   recommended option when the evidence supports one; do not author a revised
   plan until the parent returns the user's answer.
4. Otherwise, apply only that single improvement and return the complete updated
   plan. Preserve every unaffected decision and the plan's existing scope.
5. If no load-bearing assumption or plan-level contract remains, return a plain
   diminishing-returns result instead of inventing a cosmetic edit, restating
   existing content, or promoting implementation minutiae into the plan.

## Present and stop

If the author returns a genuine decision fork, ask exactly that one question
through the harness-native question mechanism and wait. After the user answers,
resume or relaunch a confirmed route under the same dispatch contract so the
strong author—not the parent—authors the complete revised plan.

Only after the foreground assignment has completed and returned its substantive
result, re-present the strong author's complete updated plan through the
harness-native approval mechanism. End the pass and wait for approval. Do not
begin implementation or another sharpening pass. A rejection or requested
revision is feedback on the current pass, not approval to continue the loop.

Repeat only after approval, starting from the approved updated plan and running
the dispatch contract again. Each approved pass may introduce only one
highest-leverage clarification. If a later pass reaches diminishing returns,
say that the plan is ready for execution and stop.

In read-only Plan Mode, keep both the candidate improvement and the revised plan
in conversational state. Perform no filesystem or plan-file writes. The
harness-native approval boundary does not authorize implementation, another
pass, or any persistent mutation.
