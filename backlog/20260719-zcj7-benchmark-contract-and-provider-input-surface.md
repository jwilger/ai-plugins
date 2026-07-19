---
title: Benchmark contract and provider-input surface
blocked_by: []
blocks: []
tags: [evals, benchmark, contract, final-review, scope-split]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Independently review the declarative benchmark contract, provider-input mapping, Promptfoo case surface, and contract validator split from cb43.

## Context / Why

Final-review split from 20260719-cb43 at diff hash b190d81690f3657f5230580fb083b666e86c8237. Scope: README, benchmark-inputs.cjs, benchmark.json, cases.cjs, promptfooconfig.yaml, validate-code-quality-contract.mjs.

## Acceptance criteria

- [ ] Define exactly one rust-cli-feature case, three conditions, three canonical samples, and nine expected turns.
- [ ] Bind each condition to the exact Codex provider settings and the case to the exact rendered task prompt.
- [ ] Reject malformed, duplicate, incomplete, reordered, or inconsistent contract and provider surfaces.
- [ ] Complete final review on the isolated diff with current verification evidence.

## Subtasks

## Notes / Log

- 2026-07-19: Administratively retired after maintainer review: this ticket represented recursive final-review bookkeeping for code already landed on main, not unfinished independently shippable work. Artificial dependency links and remote review branches were removed. The underlying implementation/evidence remains owned by 20260715-n6bs; guardrails will prevent recursive or synthetic review splits.
