---
name: eval-case-reporting
description: Use when assistant, skill, prompt, hook, tool, or agent behavior is surprising, unsafe, brittle, partial, or worth preserving as an eval case and eval_case_reporting is enabled.
---

# Eval-case reporting

Require `[features].eval_case_reporting = true` in
`.development-system.toml`.

Capture the smallest reproducible behavior. Scrub identities, secrets, private
client data, proprietary excerpts, authentication material, and private
transcripts. Show the complete sanitized issue preview and require explicit
user approval immediately before posting. Never post the raw interaction.
