# Stochastic Evaluation

Name the behavior claim, unit of analysis, metric, aggregation rule, minimum
meaningful effect, threshold, and artifact path before running an eval.

## Cases And Sampling

- Use nominal, boundary, expected-refusal or expected-error, partial-credit, and
  adversarial cases from production-like workflows.
- Estimate population quality with more distinct representative cases. Repeat
  the same case only for a named reliability measurement.
- Use pass@k when at least one of `k` attempts may succeed and pass^k when every
  one of `k` uses must succeed. Report the per-run pass rate alongside either.
- For prompt or model comparisons, keep cases and settings matched, score paired
  case-level outcomes, and predefine the smallest difference worth acting on.

## Scoring And Gates

- Prefer deterministic assertions for schemas, required fields, refusal
  conditions, citations, tool authorization, and approval gates.
- Use model judges only for semantic criteria that deterministic checks cannot
  express. Calibrate each rubric family against frozen human-labeled clear-pass,
  clear-fail, and borderline examples.
- Separate release gates from exploratory dashboards. Preserve machine-readable
  results and written failure reasons so each new failure category can become a
  regression case.
