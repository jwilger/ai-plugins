# Eval Scaffold Reference

Use an eval harness that a new contributor can run locally and CI can preserve
without depending on a hosted dashboard.

## Layout

- `evals/promptfoo/`: promptfoo loaders, assertions, and optional static
  configs. If config must reflect a marketplace manifest, generate it from the
  manifest during the run.
- `evals/fixtures/`: test cases, expected behavior, and adversarial inputs.
- `evals/out/`: generated JSON, HTML, and JUnit artifacts.
- `scripts/evals/run.sh`: one command for local and CI execution.
- `scripts/evals/build-site.mjs`: static dashboard aggregation.
- `site/evals/`: generated local static reports.
- Optional MCP wiring: Promptfoo's MCP server can expose eval operations to
  Codex or another MCP-capable agent, but the committed runner remains the
  release-evidence path.

## Runner Rules

- Install tools through the project's package-manager sandbox.
- Pin versions once the repo chooses a stable release cadence.
- Disable prompt response caching for provider-backed behavior evidence unless
  the explicit goal is offline result review.
- Keep pull-request checks runnable without provider secrets by validating config
  and dry-run wiring.
- Put live model/provider evals behind explicit trusted workflow conditions.
- For coding harnesses, start with native promptfoo providers. Use custom
  providers only when a canary proves the native provider cannot faithfully load
  the system under test.
- Load the same plugin set users will have in normal work. If plugins are meant
  to compose, run evals with all of them loaded, not a target plugin alone.
- Use canaries to prove plugin/skill loading. Keep behavior prompts natural so
  they measure routing and judgment rather than obedience to the eval prompt.
- Make failures actionable: show case id, behavior, expected result, actual
  result, and artifact path.
