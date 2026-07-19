---
title: Runtime provenance and regular-tree hashing
blocked_by: []
blocks: []
tags: [evals, benchmark, provenance, hashing, final-review, scope-split]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Independently review exact runtime provenance binding and bounded regular-tree hashing split from cb43.

## Context / Why

Final-review split from 20260719-cb43 at diff hash b190d81690f3657f5230580fb083b666e86c8237. Primary scope: runtime-manifest.cjs and code-quality-tree-hash.mjs; workspace rows are an input contract, not expanded ownership.

## Acceptance criteria

- [ ] Validate exact runtime rows against workspace rows, immutable input hashes, matrix hashes, and skill-composition evidence.
- [ ] Require canonical private non-overlapping runtime paths and exact ownership markers.

## Subtasks

## Notes / Log
