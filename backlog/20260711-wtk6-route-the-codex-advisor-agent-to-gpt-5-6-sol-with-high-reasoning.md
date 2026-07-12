---
title: Route the Codex advisor agent to gpt-5.6-sol with high reasoning
blocked_by: []
blocks: []
tags: [advisor, codex, model-routing, developer-experience]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Configure the intentionally Codex-only advisor agent to use gpt-5.6-sol with high reasoning and add focused evidence that both routing values are selected.

## Context / Why

plugins/advisor/agents/advisor.toml currently selects gpt-5.5 with high reasoning. The user explicitly chose gpt-5.6-sol with high reasoning for high-value advisor work. The advisor plugin is Codex-only, so do not add Claude routing or revive the superseded older ticket's Claude-support requirement. Keep source configuration, README claims, marketplace metadata, and the required plugin version bump consistent.

## Acceptance criteria

- [ ] Codex advisor-agent invocations select gpt-5.6-sol.
- [ ] Codex advisor-agent invocations use high reasoning effort, with focused tests or eval evidence.
- [ ] The advisor agent source config, README, Codex marketplace metadata, and semver bump consistently describe the selected model and reasoning level without adding Claude support.
- [ ] Focused source/config validation proves the exact model and effort values, with an observable runtime selection check where the Codex harness exposes one.

## Subtasks

## Notes / Log
