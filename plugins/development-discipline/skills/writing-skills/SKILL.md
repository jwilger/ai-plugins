---
name: writing-skills
description: Use when creating or editing skills in this marketplace or preparing skill behavior fixtures.
---

# Writing Skills

Skills are operational instructions for future agents. In this marketplace they
must be concise, triggerable, progressively disclosed, and backed by behavior
fixtures when the behavior matters.

## Marketplace Shape

- Put skills at `plugins/<plugin>/skills/<skill-name>/SKILL.md`.
- Keep component directories at the plugin root, not inside `.claude-plugin/` or
  `.codex-plugin/`.
- For dual-harness plugins, keep Claude and Codex plugin versions aligned and
  register the plugin in both marketplace manifests.
- Add catalog rows and a plugin README that state the supported harnesses.

## Skill Shape

- Frontmatter has `name` and a trigger-only `description` beginning with
  `Use when...`.
- The description says when to load the skill, not the full process.
- The body starts with the core rule, then the smallest useful workflow.
- Prefer tables and short checklists over long essays.
- Put heavy references or scripts in supporting files only when they are worth
  loading on demand.
- Do not import upstream workflow skills as hidden dependencies unless the
  plugin explicitly requires them.

## Behavior Fixtures

Add fixtures under `evals/fixtures/behavior/.../cases.json` when a skill changes
agent behavior that should regress visibly. Good fixtures include:

- A natural prompt that should trigger the skill.
- `plugins` and `skills` mappings.
- A semantic rubric with pass/fail criteria.
- Calibration examples for pass and fail.
- Hard assertions only for deterministic unsafe intent, not phrase matching.

## Checklist

1. Name the behavior the skill should cause.
2. Add or update a focused fixture before relying on prose.
3. Write the smallest skill text that would change that behavior.
4. Run JSON, formatting, fixture, and relevant eval dry-run checks.
5. Report provider-backed evals separately if they were not run.
