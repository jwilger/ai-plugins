---
title: Help designs avoid hidden production risks
blocked_by: []
blocks: []
tags: [engineering-standards, production-risk, reliability, evals]
pr_mr_url: 
pr_mr_status: 
claim:
  host: unknown
  session: unknown
---

## Summary

Add practical engineering guidance for risks that often appear only under real use, such as partial failures, uncontrolled retries, stale caches, cleanup problems, resource growth, and traffic spikes. The guidance should stay proportionate to how the software is actually deployed.

## Context / Why

Implementation notes: development-discipline final review and lightweight TDD review already include a production-risk-footguns lens. The remaining responsibility is the reusable engineering standard that shapes design before review. Cover partial failure, retry/loop bounds, contention, cache and cleanup hazards, N+1/fanout/resource growth, and thundering herds, but derive findings from the actual deployment and trust boundary. For a local single-owner tool, focus on mistakes, crashes, interruption, stale state, filesystem failure, and remote data loss rather than malicious local processes or intentional bypass.

## Acceptance criteria

- [ ] engineering-standards guidance explicitly reviews for hidden footguns, unsafe defaults, partial failure states, unbounded retries, unbounded loops, lock contention, cache staleness, and cleanup hazards.
- [ ] Guidance explicitly asks whether data access patterns, N+1 work, fanout, memory/IO growth, and thundering-herd behavior will survive production-sized use or DOS-like bursts.
- [ ] The change includes eval cases where an agent must flag dev/test-safe but production-risky implementation choices.
- [ ] Guidance derives risk from the intended deployment and trust boundary, with separate fixtures showing proportionate analysis for a local single-owner tool and for a shared service handling untrusted input.
- [ ] The engineering standard cross-references the existing development-discipline production-risk lens and does not duplicate that workflow's review mechanics.

## Subtasks

## Notes / Log

- 2026-07-21: Started in linked worktree .worktrees/hcq6-production-risk-lens on branch feat/hcq6-production-risk-lens. Preflight scope: engineering-standard behavior, proportionate local-tool/shared-service eval fixtures, synchronized version/catalog metadata, and plugin documentation. Delivery mode: direct-to-trunk per repository policy.
