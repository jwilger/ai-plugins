# Evals and context budget (skills, MCP, hooks)

**Effectiveness is eval-driven, not vibes.** No skill, command, or MCP-tool
description ships until an **eval** validates it — triggering accuracy _and_
behavioral effectiveness, with variance analysis (use the `skill-creator`
tooling). Re-run the eval whenever the description changes. "Looks good" is not a
passing condition.

**Minimum-necessary context.** Skills, MCP schemas/descriptions, hooks, and any
injected context must use the **least context that stays effective** across Claude
Code, Codex, and future harnesses: progressive disclosure, triggers-only
descriptions, reference material loaded on demand. Measure each surface's token
footprint in both harnesses; reject regressions that don't buy proportional
effectiveness.
