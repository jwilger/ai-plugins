---
name: eval-case-reporting
description: Use when assistant, skill, prompt, hook, tool, or agent behavior is surprising, unsafe, brittle, partial, or worth preserving as an eval case and eval_case_reporting is enabled.
---

# Eval-case reporting

Load the retained [submission and scrubbing
contract](../../components/eval-case-reporter/skills/submit-eval-case/SKILL.md)
before drafting or posting a case.

Require `[features].eval_case_reporting = true` in
`.development-system.toml`. State that requirement explicitly before doing any
reporting work; if the feature is disabled or the configuration is unavailable,
do not activate the workflow.

Capture the smallest reproducible behavior. Apply data minimization, secret
redaction, and de-identification to direct identifiers, linkable
quasi-identifiers, private client data, proprietary excerpts, authentication
material, private transcripts, internal paths, and artifact metadata. Preserve
only safe reproduction provenance and the assertion or rubric.

Classify the observed result (`pass`, `fail`, `partial`, or `uncertain`)
separately from the coverage kind (`regression`, `boundary`, `adversarial`, or
`other`). In the same response, draft and show the exact final sanitized issue
from the safe facts already available; do not merely offer to prepare it later
or ask for a scrubbed transcript. Require explicit approval for that exact body
immediately before posting, and request approval again after any change. Never
post or echo the raw interaction or removed secret values.
