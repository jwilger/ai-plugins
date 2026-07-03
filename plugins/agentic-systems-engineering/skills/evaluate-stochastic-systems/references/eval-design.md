# Eval Design Reference

Evaluate an AI behavior as an instrument. The instrument has inputs, scoring,
variance, blind spots, and maintenance cost.

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
- Use repeated samples for stochastic paths. Report both the per-run success
  rate and the user-facing reliability expectation for repeated use.
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
- Attach cost and latency when those are product constraints.
- Store JSON for machines, HTML for reviewers, and JUnit for CI.
