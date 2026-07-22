---
title: Use GPT-5.6 Sol with high reasoning for Codex advisor work
blocked_by: []
blocks: []
tags: [advisor, codex, model-routing, developer-experience]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Configure the Codex advisor to use GPT-5.6 Sol with high reasoning for high-value advisory tasks. Keep configuration, documentation, marketplace metadata, and tests consistent, and fail visibly if that model is unavailable instead of silently using a weaker one.

## Context / Why

plugins/advisor/agents/advisor.toml currently selects gpt-5.5 with high reasoning. The user explicitly chose gpt-5.6-sol with high reasoning for high-value advisor work. The advisor plugin is Codex-only, so do not add Claude routing. The superseded ticket's generic runtime-configurability and fallback requirements are intentionally not carried forward: this task pins the exact source configuration, adds no custom override layer, and must fail visibly rather than silently downgrade if the requested model is unavailable. Keep source configuration, README claims, marketplace metadata, and the required plugin version bump consistent.

## Acceptance criteria

- [x] Codex advisor-agent invocations select gpt-5.6-sol.
- [x] Codex advisor-agent invocations use high reasoning effort, with focused tests or eval evidence.
- [x] The advisor agent source config, README, Codex marketplace metadata, and semver bump consistently describe the selected model and reasoning level without adding Claude support.
- [x] Focused source/config validation proves the exact model and effort values, with an observable runtime selection check where the Codex harness exposes one.
- [x] gpt-5.6-sol/high is the single source-configured route; no custom runtime fallback or override layer is added, and model unavailability fails visibly instead of silently downgrading advisor work.

## Subtasks

## Notes / Log

- 2026-07-22: Delivered commit f6ed568361992542a5297dd189260e6affdcbd77 directly to main. Advisor 0.3.0 now pins the Codex custom advisor agent to gpt-5.6-sol with high reasoning and read-only sandboxing, removes silent fallback and caller effort overrides, reports unavailability visibly, and binds the reviewed skill contract in marketplace validation. Exact local evidence: 8 focused Advisor regressions; full just ci green with 269 development-discipline tests, 44 mutation cases (38 caught, 6 unviable), and 560 Bats tests. Independent correctness and integration re-reviews were clean. GitHub CI run 29898186314 completed successfully, including the aggregate CI gate. The formal-review coordinator's repeated current-assignment mismatch is separately tracked by 20260722-k9c6-keep-final-review-risk-assessments-linked-to-the-correct-review.
