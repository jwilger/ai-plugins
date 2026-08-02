---
name: development-workflow
description: Use for repository changes, debugging, review feedback, verification, final review, CI recovery, or deciding the next development lifecycle step.
---

# Development workflow

Read `.development-system.toml` before choosing a workflow. Treat it as the
project's single feature and delivery-policy source.

## Detailed lifecycle routing

This shared skill carries the decision-critical lifecycle route. If the
installed detailed component skill at
`components/development-discipline/skills/development-workflow/SKILL.md`
(`development-discipline:development-workflow`) is accessible, read it for
full specialist mechanics; inability to locate it must never delay or replace
the immediate lifecycle answer.

Route the active phase to its named lifecycle specialist rather than replacing
it with this checklist:

- diagnosis: `development-discipline:systematic-debugging`;
- change preparation: `development-discipline:change-preflight`, then
  `development-discipline:delivery-workflow`;
- implementation: `development-discipline:test-driven-development`;
- verification and readiness: `development-discipline:verification-before-completion`
  and `development-discipline:final-review`;
- review feedback: `development-discipline:receiving-code-review`;
- delivery: `development-discipline:delivery-workflow` and, for an authorized
  commit, `development-discipline:rationale-commit-messages`;
- unexpected pushed CI failure: `development-discipline:ci-failure-follow-up`,
  the lifecycle specialist that owns the Beads `ci-recovery` molecule.

For an advisory diagnosis, state this route and its evidence in the same
response; never answer only with a wait or delegation status. Start with
`systematic-debugging`: inspect current repository and runtime evidence,
reproduce the symptom, trace its failure path, compare relevant recent or
working evidence, and form and test one hypothesis at a time without guessing
or editing production code. Once a cause is established, re-inspect current
state, transition through `change-preflight`, choose
`delivery-workflow` before preservation actions, use
`test-driven-development` for the smallest causal repair, then perform
claim-scoped verification and final review. Do not invoke
`ci-failure-follow-up` unless a pushed CI failure caused the hold.

When `[features].beads = true`, load the `beads` skill and use the configured
Beads delivery formula rather than treating this checklist as a separate state
machine.

Before finalizing a plan with two or more dependent implementation steps,
invoke the public `advisor` skill unless an existing executable BDD-style
scenario already covers the observable behavior and material failure boundary.
If already running as Advisor or `sharpen-plan`, perform the check directly and
do not invoke Advisor recursively. Apply the resulting recommendation before
presenting the complete plan.

Before creating or substantially revising human-consumable prose,
documentation, instructions, imagery, or UI/UX, route the artifact through the
public `content-authoring` skill. It delegates creation to the
highest-capability eligible writable agent at high effort, selected explicitly
from authoritative current-harness capability or upgrade metadata. Do not pin
a strongest model or infer capability from names, list order, price, or date.
Routine status updates, ticket metadata, commit messages, and mechanical
summaries stay on their normal routes. A content author may use an authorized
specialized image tool, but model or tool selection grants no additional
authority. If capability ranking, explicit selection, high-effort writable
launch, or the required specialized tool is unavailable, report the blocked
route visibly rather than drafting on a weaker route or silently falling back.

For a change:

1. Classify each slice as runtime behavior, documentation, CI workflow, or
   validation-only before editing.
2. For runtime behavior, pour `behavior-slice`: write and run the executable
   acceptance scenario first, repair an unexpectedly passing or wrong failure,
   implement one scenario step at a time, and pour a focused unit TDD cycle when
   the failure is not one tightly scoped semantic unit.
3. Use focused tests during micro-cycles. Run the complete configured local gate
   after the behavior slice is green, not after every tiny edit.
4. Keep diagnosis causal and implementation scoped.
5. Verify the current increment with fresh evidence proportional to its claim.
6. Only after the slice gate passes, commit and push using the configured
   delivery mode.

Choose verification from the release claim, not from the fact that prose or a
skill changed. Retirement, deletion, exact file or routing state, schemas,
formatting, metadata, and mechanical transformations are deterministic
contracts and do not warrant provider evals when deterministic checks fully
decide them. Before any provider eval, name the unresolved stochastic question
and record why deterministic evidence is insufficient. Require clean,
fail-closed isolation between baseline and treatment and an explicit
incremental-value hypothesis for any lift gate. An unexpectedly successful
baseline triggers an isolation, context, prompt, and rubric audit followed by a
deliberate case retirement or documented absolute-reliability disposition;
never bypass that diagnosis by blindly selecting `valueGate.mode: none`.

For each intermediate green checkpoint, run fast unit tests and directly
relevant quick checks plus lightweight review, then commit and push as the
selected delivery mode allows. Long-running integration, mutation, exhaustive,
and full-suite evidence belongs in CI unless a local run is needed to diagnose
a failure. Start full review only after the actual acceptance criteria are
implemented. If full review requires an edit, first recheck checkpoint
eligibility, make the edit test-first, repeat the fast checks, lightweight
review, commit, and permitted push, then submit one delta risk assessment and
resume only affected or guard assignments.

In direct-to-trunk mode, another verified semantic checkpoint may be pushed
when the most recent completed run for the configured trunk branch that reached
a pass/fail outcome passed and no unresolved failure hold exists. Newer queued,
pending, or running runs do not replace that watermark. A canceled run is
non-evidence: it neither passes nor fails, does not replace the watermark, and
does not create or release a hold. Do not confuse checkpoint eligibility with
ticket completion, which requires terminal success for the exact final pushed
SHA.

The instant a completed run for the configured trunk branch fails unexpectedly,
stop unrelated work and route immediately to
`development-discipline:ci-failure-follow-up`; do not use this checklist as a
recovery procedure. The hold permits only causal recovery until terminal
success of either the exact tested causal-repair revision or the authorized
rerun of the exact unchanged failed SHA. When `[features].beads = true`, that
specialist owns the atomically claimed `ci-recovery` molecule and Beads merge
slot. An intentional failure while testing an active `ci-workflow-slice`
remains related work in that slice rather than a separate incident. When Beads
is disabled, preserve the same single-owner hold and recovery evidence locally.

Load [workflow rules](references/workflow-rules.md) only when executing or
reviewing a change.
