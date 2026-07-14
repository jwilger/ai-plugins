---
title: Apply the final-review relevance gate to risk-scout findings
blocked_by: []
blocks: []
tags: [development-discipline, final-review, correctness, relevance, major]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Route initial and delta risk-scout findings through the same deterministic relevance gate as normal lens findings so generic observations cannot pollute or block the backlog.

## Context / Why

Final-review correctness finding from 20260713-rygd. This is not covered by 20260714-jyu9, which supplies conditional-lens objectives but does not enforce relevance evidence; 20260714-24xa covers split/budget contracts; 20260714-ra58 strengthens prose-fixture semantics. This ticket owns the production relevance gate for all initial and delta scout findings.

## Acceptance criteria

- [ ] Initial and delta risk-scout findings pass through the same deterministic relevance validation and filtering used for normal lens findings before persistence, disposition, blocker calculation, or follow-up-ticket requirements.

## Subtasks

## Notes / Log
