# Eval Design Reference

Treat an AI eval as a measurement system with a stated behavior claim, unit of
analysis, metric, aggregation rule, variance, blind spots, and maintenance cost.

## Eval Set Shape

- Start with real production-like cases where possible.
- Add synthetic cases only after a human reviews them for realism and correct
  expected behavior.
- Cover nominal, boundary, expected-refusal or expected-error, partial-credit,
  and adversarial cases.
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
- For prompt or model comparisons, keep cases and settings matched, score paired
  case-level outcomes, and predefine the minimum meaningful effect: the smallest
  difference worth acting on.
- Do not treat close scores as meaningful without enough cases to distinguish
  that effect from sampling variation.

## Scoring

- Prefer deterministic assertions for contracts: schema, required fields,
  refusal conditions, citation presence, tool authorization, and safety gates.
- Use rubric or judge scoring for semantic quality only when deterministic
  checks cannot represent the behavior.
- Calibrate each rubric family with frozen human-labeled clear-pass, clear-fail,
  borderline, and prompt-injection-style examples.
- Preserve written critiques for failures so future maintainers understand why
  a case exists.

## Gates

- Define thresholds before the run.
- Separate release gates from investigation dashboards.
- Attach cost and latency when those are product constraints.
- Store JSON for machines, HTML for reviewers, and JUnit for CI.
