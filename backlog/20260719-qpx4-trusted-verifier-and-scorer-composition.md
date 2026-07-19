---
title: Trusted verifier and scorer composition
blocked_by: []
blocks: []
tags: [evals, benchmark, scoring, final-review, scope-split]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Review and independently disposition the bounded trusted verifier/scorer composition extracted from the zcsh final-review scope split.

## Context / Why

Created from final-review session final-review-zcsh-final-20260718 at diff hash 54ece87326cb708bd8d46f1c25a3bbeeef69884a. Scope: assertion adapter, verifier modules, scorer security/composition tests, and adversarial Rust fixtures. This slice owns sandboxed rebuilding, deterministic gates, and trusted scoring independently of result publication and live-runner orchestration.

## Acceptance criteria

- [ ] Rebuild and score candidate code in bounded trusted sandboxes and bind the exact scoring composition and fixture.
- [ ] Enforce black-box behavior, regression replay, formatting, Clippy, diff-scope, and safety gates; reject unsafe or inconsistent evidence.
- [ ] Add direct regression coverage proving duplicate raw-result artifacts are rejected.
- [ ] Complete final review against the isolated slice with current diff-bound verification evidence.

## Subtasks

## Notes / Log

- 2026-07-19: Administratively retired after maintainer review: this ticket represented recursive final-review bookkeeping for code already landed on main, not unfinished independently shippable work. Artificial dependency links and remote review branches were removed. The underlying implementation/evidence remains owned by 20260715-n6bs; guardrails will prevent recursive or synthetic review splits.
