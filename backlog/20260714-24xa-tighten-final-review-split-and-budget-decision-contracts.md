---
title: Tighten final-review split and budget decision contracts
blocked_by: []
blocks: []
tags: [development-discipline, final-review, mcp, contracts, minor, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make final-review split plans meaningfully independent and make the published budget-decision schema match runtime acceptance exactly.

## Context / Why

Consolidates two MINOR observations from the risk-proportionate final-review ticket. Split candidates currently need collective changed-file coverage but may fully overlap, weakening the decomposition signal. Budget decision JSON Schema and runtime validation also apply different length/field constraints, so a payload can be accepted by one layer and rejected by the other. Value: clearer reliable coordinator contracts. Risk/impact: low-to-moderate workflow friction rather than release corruption. Likelihood: possible for generated callers and broad split plans. Opportunity cost: lower than current common product/tooling defects.

## Acceptance criteria

## Subtasks

## Notes / Log
