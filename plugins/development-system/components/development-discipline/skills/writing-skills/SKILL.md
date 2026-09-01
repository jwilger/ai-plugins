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
- Keep component directories at the plugin root, not inside `.codex-plugin/`.
- Keep each Codex plugin manifest version aligned with its marketplace entry.
- Add a catalog row and a plugin README that describe the supported behavior.

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

## Instruction Language

Use the canonical domain term when it distinguishes behavior, and define its
operational predicate where the instruction first relies on it. Prefer
"authentication boundary changes when an untrusted principal gains a new path
to an asset" over either "security becomes important" or an ornamental synonym.

Every behavior-changing instruction must make these parts discoverable:

- the trigger and required inputs or evidence;
- eligibility and exclusion predicates for any route or exemption;
- the operation to perform and its observable result;
- the stop, blocked, fallback, and authorization boundaries.

If an adjective such as `relevant`, `material`, `adequate`, `appropriate`,
`simple`, `broad`, `safe`, or `fresh` changes routing, scope, or a gate, replace
it with an observable predicate or an explicit evidence requirement. Keep plain
connective prose around the necessary technical vocabulary; do not increase
reading difficulty merely to sound formal.

## Behavior Fixtures

Add fixtures under `evals/fixtures/behavior/.../cases.json` when a skill changes
agent behavior that should regress visibly. Good fixtures include:

- A natural prompt that should trigger the skill.
- `plugins` and `skills` mappings.
- A semantic rubric with pass/fail criteria.
- Calibration examples for pass and fail.
- Hard assertions only for deterministic unsafe intent, not phrase matching.
- At least one positive case and one close-boundary negative case for every new
  routing, exemption, stop, or authorization predicate.

## Checklist

1. Name the behavior contract: trigger, predicates, action, evidence, and stop
   condition.
2. Add or update a focused fixture before relying on prose.
3. Write the smallest skill text that would change that behavior.
4. Run JSON, formatting, fixture, and relevant eval dry-run checks.
5. Report provider-backed evals separately if they were not run.
