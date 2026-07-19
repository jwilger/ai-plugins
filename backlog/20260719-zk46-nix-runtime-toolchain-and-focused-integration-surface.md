---
title: Nix runtime toolchain and focused integration surface
blocked_by: []
blocks: [20260718-zcsh-review-benchmark-contract-and-scorer-slice]
tags: [nix, devshell, evals, final-review, scope-split]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Review and independently disposition the pinned Nix runtime toolchain surface extracted from the zcsh final-review scope split.

## Context / Why

Created from final-review session final-review-zcsh-final-20260718 at diff hash 54ece87326cb708bd8d46f1c25a3bbeeef69884a. Scope: flake.nix, flake.lock, and devshell runtime-tool verification. This reproducible dependency surface can be validated before canonical live execution is connected.

## Acceptance criteria

- [ ] Provide the pinned runtime tools and closure identities required by the benchmark contract and scorer.
- [ ] Verify the repository devshell exposes the expected runtime-tool surface without global or unpinned substitutions.
- [ ] Complete final review against the isolated slice with current diff-bound verification evidence.

## Subtasks

## Notes / Log
