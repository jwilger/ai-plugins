# Evals and context budget (skills, MCP, hooks)

**Effectiveness is eval-driven, not vibes.** No skill, command, or MCP-tool
description ships until an **eval** validates it — triggering accuracy _and_
behavioral effectiveness, with variance analysis (use the `skill-creator`
tooling). Re-run the eval whenever the description changes. "Looks good" is not a
passing condition.

**Minimum-necessary context.** Skills, MCP schemas/descriptions, hooks, and any
injected context must use the **least context that stays effective** across
Codex: progressive disclosure, triggers-only descriptions, and reference
material loaded on demand. Measure model-facing effectiveness and token
footprint through Codex; reject context regressions that do not buy
proportional effectiveness.

## Match evaluation to the causal surface

Provider-backed evaluation is evidence for stochastic, model-mediated claims,
not a generic tax on every file below a plugin directory. Require live LLM
evals when instructions, triggers, model-visible schemas or results, injected
context, prompt construction, routing, or graders can change what a model does.
Scope cases and samples to the claim being measured. This repository's live
behavior evals use Codex.

Use deterministic tests for deterministic behavior such as installation,
packaging, process lifecycle, locking, paths, state directories, permissions,
manifest synchronization, and non-instructional documentation. A wiring dry
run may prove that an eval harness still composes, but it is not behavior
evidence and does not justify spending provider quota when no model-mediated
behavior changed. Record the applicable deterministic evidence and mark live
behavior evals not applicable rather than running them reflexively.
