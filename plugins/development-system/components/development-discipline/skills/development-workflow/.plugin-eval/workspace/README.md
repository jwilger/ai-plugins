# Development workflow benchmark workspace

The benchmark asks Codex to write `workflow-plan.json` for three lifecycle
routing scenarios. It must not edit repository content or contact a remote.

`verify-workflow-plan.mjs` validates the plan selected for the current scenario.
