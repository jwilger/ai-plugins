---
title: Keep live evaluation approval consistent across providers
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Make every repository rule and guardrail agree that the owner has already approved live, repository-owned evaluations through both supported model providers using the owner's existing subscription authentication. This prevents authorized quality checks from stopping for unnecessary approval requests while preserving restrictions against sending secrets or unrelated private material.

## Context / Why

The current repository guidance was narrow enough that an approval reviewer interpreted standing approval as covering only Codex/OpenAI. The owner clarified that standing approval is symmetric: it covers Codex/OpenAI through the existing ChatGPT subscription authentication and Claude/Anthropic through the existing Anthropic subscription authentication. Review every repository rule, execution policy, approval policy, skill, hook, script, documentation page, and behavior fixture that describes or enforces live-evaluation authorization. Make them consistent, name both providers explicitly, and avoid requiring API keys when the supported coding harness can reuse the owner's existing authenticated CLI session. Preserve isolated/disposable evaluation state, leave source logins untouched, run secret-leak checks, exclude protected or unrelated data, handle provider credentials safely, and keep restrictions on untrusted events.

## Acceptance criteria

- [x] Repository guidance explicitly says standing approval covers both Codex/OpenAI and Claude/Anthropic live evaluations.
- [x] Execution and approval guardrails do not request fresh approval solely because an authorized repository evaluation uses either named provider.
- [x] Existing protections for secrets, private data, unrelated files, isolated authentication, and untrusted events remain in force.
- [x] Automated tests or behavior fixtures fail if future guidance drops either provider or reintroduces unnecessary approval prompts.
- [x] All affected plugin versions, marketplace metadata, documentation, and evaluation coverage are updated as required by repository policy.
- [x] Authorized live evaluations reuse the existing authenticated Codex and Claude coding-harness sessions when supported; they do not require provider API keys merely because a live evaluation is being run.

## Subtasks

## Notes / Log

- 2026-07-20: Owner reconfirmed on 2026-07-20 that standing authorization is symmetric: Codex/OpenAI via the existing ChatGPT subscription-authenticated coding harness and Claude/Anthropic via the existing Anthropic subscription-authenticated coding harness. Audit found AGENTS.md's generic standing-authorization section currently names only Codex/ChatGPT/OpenAI and must be corrected. Provider-specific Codex benchmark docs/scripts should remain provider-specific rather than falsely implying Anthropic execution. Audit every actual approval/execution guardrail and add regression coverage that requires both provider pairs.
- 2026-07-21: Completed on main in commits 51f2b63 and 9751808. Repository, plugin, skill, runner-help, marketplace, and behavior guidance now names both subscription-authenticated provider families while preserving repository-owned-data limits, isolated/disposable generated authentication state, untouched source logins, secret-leak checks, and untrusted-event restrictions. Deterministic tests mutation-check provider and boundary removal. Verification: 44 focused Bats tests passed; full local CI passed all 584 Bats tests plus Rust, mutation, release, manifest, formatting, and actionlint gates; directly relevant live cases passed across Claude and Codex with clean secret scans; final review completed clean. Exact pushed CI runs 29800760160 (51f2b63) and 29801939493 (9751808) both completed successfully.
