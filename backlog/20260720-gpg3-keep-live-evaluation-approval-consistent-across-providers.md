---
title: Keep live evaluation approval consistent across providers
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make every repository rule and guardrail agree that the owner has already approved live, repository-owned evaluations through both supported model providers using the owner's existing subscription authentication. This prevents authorized quality checks from stopping for unnecessary approval requests while preserving restrictions against sending secrets or unrelated private material.

## Context / Why

The current repository guidance was narrow enough that an approval reviewer interpreted standing approval as covering only Codex/OpenAI. The owner clarified that standing approval is symmetric: it covers Codex/OpenAI through the existing ChatGPT subscription authentication and Claude/Anthropic through the existing Anthropic subscription authentication. Review every repository rule, execution policy, approval policy, skill, hook, script, documentation page, and behavior fixture that describes or enforces live-evaluation authorization. Make them consistent, name both providers explicitly, and avoid requiring API keys when the supported coding harness can reuse the owner's existing authenticated CLI session. Preserve isolated/disposable evaluation state, leave source logins untouched, run secret-leak checks, exclude protected or unrelated data, handle provider credentials safely, and keep restrictions on untrusted events.

## Acceptance criteria

- [ ] Repository guidance explicitly says standing approval covers both Codex/OpenAI and Claude/Anthropic live evaluations.
- [ ] Execution and approval guardrails do not request fresh approval solely because an authorized repository evaluation uses either named provider.
- [ ] Existing protections for secrets, private data, unrelated files, isolated authentication, and untrusted events remain in force.
- [ ] Automated tests or behavior fixtures fail if future guidance drops either provider or reintroduces unnecessary approval prompts.
- [ ] All affected plugin versions, marketplace metadata, documentation, and evaluation coverage are updated as required by repository policy.

## Subtasks

## Notes / Log
