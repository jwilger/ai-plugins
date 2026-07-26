---
name: eval-case-reporting
description: Use when assistant, skill, prompt, hook, tool, or agent behavior is surprising, unsafe, brittle, partial, or worth preserving as an eval case and eval_case_reporting is enabled.
---

# Eval-case reporting

Require `[features].eval_case_reporting = true` in
`.development-system.toml`. State that requirement explicitly before doing any
reporting work; if the feature is disabled or the configuration is unavailable,
do not activate the workflow.

Capture the smallest reproducible behavior. Scrub identities, secrets, private
client data, proprietary excerpts, authentication material, and private
transcripts. In the same response, draft and show the complete sanitized issue
preview from the safe facts already available; do not merely offer to prepare it
later or ask for a scrubbed transcript. Require explicit user approval
immediately before posting. Never post the raw interaction.
