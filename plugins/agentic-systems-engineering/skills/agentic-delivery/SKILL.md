---
name: agentic-delivery
description: Use when planning or delivering an LLM or agentic-system project, especially for experiment loops, walking skeletons, demos, data stories, scope control, and release-readiness evidence.
---

# Agentic Delivery

Use this skill when the work is uncertain because model behavior, data quality,
workflow fit, or human trust must be discovered.

Load `references/delivery.md`.

## Practice

- Start with a walking skeleton: one real workflow path, minimal tools, visible
  traces, and enough evals to compare changes.
- Run an experiment loop: baseline, inspect failures, change one thing, rerun,
  compare against thresholds, and decide promote or iterate.
- Keep two tracks visible: product workflow discovery and technical reliability
  evidence.
- Pair demos with data stories. A demo shows the workflow; the data story shows
  rate-over-set quality, cost, latency, and the failures still outside scope.
- Include at least one failure-path case in stakeholder evidence so the review
  is not a happy-path-only performance.
- Use stakeholder reviews to decide scope and risk, not to launder a happy path
  into a release claim.
