---
title: Update all evals to use gpt-5.6 model family when running against Codex
blocked_by: []
blocks: []
tags: [evals, codex, model-routing, benchmarking, major]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Benchmark the GPT-5.6 family for Codex eval execution and grading roles, then migrate every Codex eval surface to an evidence-backed standard-run/grader split while retaining explicit overrides.

## Context / Why

The canonical Codex eval matrix, runner help, README, and semantic grader currently default to gpt-5.5. Evaluate GPT-5.6 Terra at medium reasoning as the standard-run candidate and GPT-5.6 Sol at high reasoning as the grader candidate, while verifying exact current identifiers and availability from official documentation before implementation. Representative advisor-like eval scenarios belong in the benchmark population, but this task does not select or configure the installed advisor plugin's agent; 20260711-wtk6 is the canonical fixed gpt-5.6-sol/high advisor routing decision. Measure quality, latency, and token/cost behavior and document whether the grader should remain independent from the model under test. Claude providers and their defaults are out of scope.

## Acceptance criteria

- [x] The eval matrix, Codex provider configuration, semantic grader, runner help, README, generated-config tests, and dashboard/site labels are updated with no stale default gpt-5.5 references.
- [x] Existing environment overrides remain supported and Claude provider configuration is unchanged.
- [x] Generated-config and dry-run coverage passes, plus a focused provider-backed sample when credentials and approval are available; any unavailable live evidence is stated explicitly.
- [x] A documented benchmark compares supported GPT-5.6 candidates on representative standard eval cases and advisor-like eval scenarios using pass rate, latency, token usage, and cost, after verifying exact current model identifiers.
- [x] The selected Codex eval execution and grader model/reasoning split is justified from the benchmark, including grader independence; it does not alter the installed advisor agent governed by 20260711-wtk6.

## Subtasks

## Notes / Log

- 2026-07-14: 2026-07-14 completion audit: pushed HEAD 464e8b57dd242cf779c6c429c559f34f654734ef matches origin/main and GitHub Actions run 29304280229 completed successfully (eval-config dry-run, cross-harness manifests, full quality gate, CI gate). Source inspection confirms Terra/medium execution and Sol/high grading defaults, preserved env overrides and Claude defaults, documented Sol/Terra/Luna standard plus advisor-like benchmark evidence with pass/latency/token/cost data, explicit current live-evidence unavailability, grader independence, and no installed Advisor routing change. One medium-risk formal pass completed all eight lenses with no caused CRITICAL or MAJOR findings. Deferred MINORs remain consolidated in 20260713-2rd3, 20260714-yevb, 20260713-uf3e, 20260713-dcww, 20260714-e3sx, and 20260714-8n76. The current final-review machinery incorrectly marks relevant MINOR findings as blockers and demanded a routine verifier after explicit backlog dispositions; per the authoritative risk-proportionate policy, no verifier or extra pass was run. That tooling defect will be included in the next top-priority development-discipline policy ticket.
