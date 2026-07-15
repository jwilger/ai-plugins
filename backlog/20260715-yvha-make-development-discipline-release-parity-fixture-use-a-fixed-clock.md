---
title: Make development-discipline release parity fixture use a fixed clock
blocked_by: []
blocks: [20260712-4qmz-reject-option-tokens-consumed-as-missing-tiber-option-values-before-writes]
tags: [development-discipline, tests, release, hermeticity, ci, major, backlog]
pr_mr_url: 
pr_mr_status: 
---

## Summary

Make the source-versus-bundled development-discipline MCP parity check deterministic by giving both fixture runs the same clock value.

## Context / Why

The canonical `just ci` gate compares complete JSON-RPC outputs from sequential source and bundled MCP runs. Each run currently captures the wall-clock second in `risk_plan.review_budget.started_at_epoch_seconds`; crossing a one-second boundary changes that field and the derived review contract ID, causing `development-discipline-release-parity-mismatch=true` even when both binaries behave identically. The failure reproduced twice while Tiber-only gates remained green.

## Acceptance criteria

- [ ] The source and bundled MCP fixture runs receive the same deterministic epoch value, independent of wall-clock second boundaries.
- [ ] The release-from-source parity gate passes repeatedly when source and bundled behavior is identical while still failing for a real output difference.
- [ ] Focused coverage reproduces the former clock-derived mismatch and proves the deterministic fixture contract.

## Subtasks

## Notes / Log
