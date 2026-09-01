---
name: evaluate-stochastic-systems
description: Use when evaluating prompts, LLM features, agents, RAG, judges, tool-use policies, or any stochastic behavior where one successful run is not reliable evidence.
---

# Evaluate Stochastic Systems

Use this skill before claiming an LLM-backed behavior is correct, reliable, or
ready.

## Required Discipline

Load `references/eval-design.md`.

- Measure rates over an eval set, not anecdotes.
- Include nominal, boundary, expected-refusal or expected-error, partial-credit,
  and adversarial cases.
- Prefer more distinct representative cases for population quality. Repeat the
  same case only for a named metric such as per-input reliability, pass@k
  capability, pass^k reliability, or judge variance.
- Set thresholds before running the eval.
- Track regressions by adding cases for every meaningful failure category.
- When a new reusable failure appears in this marketplace, suggest filing a
  repo-level GitHub **Eval case** issue. If the `eval-case-reporting` skill is
  available, use it so the case is scrubbed,
  previewed, approved, and posted consistently.
- Keep deterministic checks for contract behavior and calibrated judgment for
  semantic quality.
- Refuse to treat one successful run, one demo, or one hand-picked example as
  proof.

## Live Provider Runs

- Follow the repository's explicit authorization policy. When it grants standing
  approval for repository-owned evals through existing Codex/ChatGPT
  subscription sessions, reuse those sessions without
  demanding provider API keys or fresh approval.
- Standing approval does not broaden the data boundary: send only the authorized
  fixtures and prompts, exclude secrets and private or unrelated content, keep
  generated authentication state isolated where supported, leave source logins
  untouched, and run required secret-leak checks.
- Keep provider credentials and live eval execution out of untrusted
  pull-request code and events. Use protected credentials for unattended trusted
  automation only when an interactive harness session is unavailable.

## Output Bar

For any eval recommendation, produce:

- The behavior under test.
- The fixture source and expected outcome.
- The unit of analysis, scoring method, and named metric.
- The distinct-case count, repeated-sample count, and aggregation rule.
- The pass threshold and release gate.
- The failure taxonomy used to grow the suite.
- The artifact path that preserves results for review.
