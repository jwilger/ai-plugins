# Eval Scaffold Reference

Use an eval harness that a new contributor can run locally and CI can preserve
without depending on a hosted dashboard.

## Layout

- `evals/promptfoo/`: promptfoo configs and local providers.
- `evals/fixtures/`: test cases, expected behavior, and adversarial inputs.
- `evals/out/`: generated JSON, HTML, and JUnit artifacts.
- `scripts/evals/run.sh`: one command for local and CI execution.
- `scripts/evals/build-site.mjs`: static dashboard aggregation.
- `site/evals/`: generated reports for GitHub Pages.

## Runner Rules

- Install tools through the project's package-manager sandbox.
- Pin versions once the repo chooses a stable release cadence.
- Keep deterministic fixtures runnable without API keys.
- Put live model/provider evals behind explicit trusted workflow conditions.
- Make failures actionable: show case id, behavior, expected result, actual
  result, and artifact path.
