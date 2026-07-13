---
title: Correct GPT-5.6 benchmark docs for superseded labels and path lifecycle
blocked_by: []
blocks: []
tags: [evals, gpt-5.6, documentation, reproducibility, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the GPT-5.6 benchmark README consistently separate six-label superseded pilot evidence from the current eight-label method and describe path cleanup/reuse exactly.

## Context / Why

Caused MINOR deferred from 20260709-spx8. The decision/limits prose still says calibration has six labels and 6/6 outside the Superseded pilot heading, while current config has eight hostile-inclusive labels. Method also says homes, workspaces, and output directories are freshly recreated; only Codex homes are recreated, the shared workspace is mkdir-p, and primary output files are cleared in persistent per-phase directories. Repair these claims or align implementation in a focused docs/lifecycle ticket.

## Acceptance criteria

## Subtasks

## Notes / Log

- 2026-07-13: Final-review iteration 1 reconfirmed the deferred MINOR: the README says phase output directories are freshly recreated, but stable directories retain auxiliary files and a preparation failure can occur before standard result files are cleared. Preserve the distinction between directory freshness and checker-verified current artifacts.
