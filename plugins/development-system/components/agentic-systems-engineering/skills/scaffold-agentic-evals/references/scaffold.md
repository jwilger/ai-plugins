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
- Ensure `promptfoo@0.121.18` is available on `PATH` before relying on
  Promptfoo commands or the optional MCP server. If the project uses
  `flake.nix` and nixpkgs provides a compatible `pkgs.promptfoo`, prefer adding
  that package there so updates flow through the flake lockfile; otherwise use
  the existing project-local package manager sandbox.
- Pin versions once the repo chooses a stable release cadence.
- Disable prompt response caching for provider-backed behavior evidence unless
  the explicit goal is offline result review.
- Keep pull-request checks runnable without provider secrets by validating config
  and dry-run wiring.
- Reuse authenticated Claude Code and Codex subscription sessions for local live
  evals without demanding provider API keys or repeat approval. When unattended
  trusted automation cannot reuse an interactive session, keep any provider
  credentials protected behind explicit trusted workflow conditions.
- Limit provider-bound inputs to purpose-built repository fixtures and prompts;
  exclude secrets, private client data, proprietary unrelated content, and
  unrelated workspace files. Keep generated authentication state isolated and
  disposable where supported, leave source harness logins untouched, and run
  required secret-leak checks around live execution.
- For coding harnesses, start with native promptfoo providers. Use custom
  providers only when a canary proves the native provider cannot faithfully load
  the system under test.
- Run the production plugin composition users receive. When attributing a
  behavior change, also run a targeted-plugin ablation against the same cases
  and provider settings.
- Use canaries to prove plugin/skill loading. Keep behavior prompts natural so
  they measure routing and judgment rather than obedience to the eval prompt.
- Record exact provider, model, reasoning settings, plugin composition,
  distinct-case count, repeated-sample count, named metric, and aggregation rule
  in the result artifact.
- Make failures actionable: show case id, behavior, expected result, actual
  result, and artifact path.
