# Eval Design Reference

Evaluate an AI behavior as an instrument. The instrument has inputs, scoring,
variance, blind spots, and maintenance cost.

## Decide Whether a Provider Eval Is Needed

Name the unresolved stochastic question before designing or running a provider
eval. A provider eval is justified only when deterministic evidence cannot
fully decide the release claim. Record why the available tests, static checks,
or inspection are insufficient and what provider evidence would resolve.

Prefer deterministic verification whenever the contract is fully decided by a
repeatable check. This includes retirement or deletion, exact file presence or
absence, routing tables, schemas, formatting, metadata synchronization, and
mechanical transformations. Skill or prompt prose changing does not by itself
require a live eval: use one only when the release claim includes unresolved
triggering, interpretation, semantic quality, tool-use, routing, or reliability
behavior from a model.

Every retained provider case must state:

- the unresolved stochastic question;
- that deterministic verification does not fully decide it; and
- the specific insufficiency that requires provider evidence.

If those statements cannot be made truthfully, retire the provider case and
verify the contract deterministically.

## Context Isolation

Fail closed when the intended baseline or treatment cannot be isolated. Give
each condition only its declared plugin, skill, prompt, tools, conversation
history, environment, and authentication context. Prove that a no-plugin
baseline did not inherit the treatment through a shared home, installed cache,
session state, injected repository instructions, prior conversation, or
generated configuration. If isolation is absent, ambiguous, or contaminated,
stop and classify the run as invalid; do not score it, waive the gate, or use it
as release evidence.

## Eval Set Shape

- Start with real production-like cases where possible.
- Add synthetic cases only after a human reviews them for realism and correct
  expected answers.
- Cover pass, fail, partial, and adversarial outcomes.
- Include boundary cases for missing context, conflicting context, unsafe
  instructions, malformed tool outputs, empty results, and timeout paths.
- Grow the set whenever a new failure category appears.
- For marketplace-wide plugin or skill failures, use the repository's **Eval
  case** GitHub issue template as the intake path before promoting a sanitized
  report into `evals/fixtures/`.

## Sampling

- One good run proves only that a path can succeed once.
- Use rate-over-set metrics for routine quality.
- Prefer more distinct, representative examples when estimating population
  quality.
- Use repeated samples deliberately for per-input reliability, pass@k capability
  (`at least one of k succeeds`), pass^k reliability (`all k succeed`),
  stochastic judge variance, or close A/B comparisons.
- Report both the per-run success rate and the user-facing reliability
  expectation for repeated use.
- Do not increase `k` as a ritual substitute for better fixtures.
- Do not compare close scores as meaningful unless the set has enough cases to
  detect that difference.

## Scoring

- Prefer deterministic assertions for contracts: schema, required fields,
  refusal conditions, citation presence, tool authorization, and safety gates.
- Use rubric or judge scoring for semantic quality only when deterministic
  checks cannot represent the behavior.
- Calibrate judges with known good, known bad, and borderline examples.
- Preserve written critiques for failures so future maintainers understand why
  a case exists.

## Gates

- Define thresholds before the run.
- Separate release gates from investigation dashboards.
- Require an explicit incremental-value hypothesis before comparing a plugin
  treatment with a baseline. A standard or safety-critical lift gate is valid
  only when the case states what improvement the treatment is expected to
  cause and why the baseline should not already satisfy the same behavior.
- Treat unexpected baseline success as a diagnostic event, not a reason to
  disable the gate. Audit condition isolation, inherited context, prompt
  leakage, and rubric or assertion specificity. Then deliberately either
  retire the case because the treatment adds no measured value, or reclassify
  it as absolute-reliability evidence with a written reason that baseline lift
  is not the claim.
- Never set `valueGate.mode` to `none` merely to make a surprising baseline or
  threshold failure pass. A no-lift disposition must identify the measurement
  or absolute-reliability claim and record the completed baseline audit.
- Attach cost and latency when those are product constraints.
- Store JSON for machines, HTML for reviewers, and JUnit for CI.
