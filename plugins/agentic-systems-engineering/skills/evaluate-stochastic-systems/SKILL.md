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
- Include pass, fail, partial, and adversarial fixtures.
- Repeat stochastic cases with `k` samples (`k samples`) when randomness, retries, routing, or
  agent planning can change outcomes.
- Set thresholds before running the eval.
- Track regressions by adding cases for every meaningful failure category.
- When a new reusable failure appears in this marketplace, suggest filing a
  repo-level GitHub **Eval case** issue. If the `eval-case-reporter` plugin is
  available, use its `submit-eval-case` skill so the case is scrubbed,
  previewed, approved, and posted consistently.
- Keep deterministic checks for contract behavior and calibrated judgment for
  semantic quality.
- Refuse to treat one successful run, one demo, or one hand-picked example as
  proof.
- A single good run is a demo, not proof.

## Live Provider Runs

- Follow the repository's explicit authorization policy. When it grants standing
  approval for repository-owned evals through existing Claude Code/Anthropic
  and Codex/ChatGPT subscription sessions, reuse those sessions without
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
- The scoring method.
- The sample count and aggregation rule.
- The pass threshold and release gate.
- The failure taxonomy used to grow the suite.
- The artifact path that preserves results for review.
