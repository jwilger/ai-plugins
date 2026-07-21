---
title: Stop measuring plugin lift when an eval prompt gives away the answer
blocked_by: []
blocks: []
tags: []
pr_mr_url: 
pr_mr_status: 
---

## Summary

Correct behavior evaluations that compare an installed plugin with a no-plugin baseline even though the prompt itself states the expected answer. Those cases should measure correctness or reliability instead of claiming the plugin caused an improvement.

## Context / Why

The existing stochastic-readiness case tells the model that one successful run is not enough, then expects the installed plugin to outperform a no-plugin model. Because the prompt supplies the key conclusion, a correct no-plugin answer is legitimate and plugin-lift is not a meaningful metric. Review similar fixtures, choose a metric that matches each case's purpose, and keep genuine baseline-ablation cases unchanged.

## Acceptance criteria

- [ ] Identify behavior cases whose prompts explicitly provide the conclusion they are scored on but still require plugin-versus-baseline lift.
- [ ] Change only those cases to a correctness or reliability metric that matches their stated purpose.

## Subtasks

## Notes / Log
